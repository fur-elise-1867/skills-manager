use super::{config::CompanyConfig, types::*};
use reqwest::blocking::{Client, Response};
use serde::de::DeserializeOwned;
use serde_json::{json, Value};
use std::{io::Read, time::Duration};

#[derive(Clone)]
pub struct StoreClient {
    pub(crate) client: Client,
    pub(crate) base_url: String,
}
impl StoreClient {
    pub fn new(config: &CompanyConfig) -> CompanyResult<Self> {
        config.validate()?;
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(120))
            .build()
            .map_err(|_| CompanyError::new("NETWORK_ERROR", "Không khởi tạo được HTTP client."))?;
        Ok(Self {
            client,
            base_url: config.store_url.clone(),
        })
    }
    pub(crate) fn endpoint(&self, path: &str) -> CompanyResult<String> {
        if self.base_url.is_empty() {
            return Err(CompanyError::new(
                "CONFIGURATION_ERROR",
                "Chưa cấu hình Skill Store.",
            ));
        }
        Ok(format!("{}/api/v1{}", self.base_url, path))
    }
    pub(crate) fn response_error(response: Response) -> CompanyError {
        let retry_after_seconds = response
            .headers()
            .get(reqwest::header::RETRY_AFTER)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.parse().ok());
        let status = response.status().as_u16();
        let fallback = match status {
            401 => "UNAUTHENTICATED",
            403 => "FORBIDDEN",
            404 => "SKILL_NOT_FOUND",
            412 => "PACKAGE_CHANGED",
            429 => "RATE_LIMITED",
            503 => "STORE_UNAVAILABLE",
            _ => "STORE_ERROR",
        };
        let mut bytes = Vec::new();
        let _ = response.take(65_536).read_to_end(&mut bytes);
        let value: Value = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
        let error = &value["error"];
        CompanyError {
            code: error["code"].as_str().unwrap_or(fallback).to_string(),
            message: error["message"]
                .as_str()
                .unwrap_or("Skill Store không thể hoàn tất yêu cầu.")
                .chars()
                .take(512)
                .collect(),
            request_id: error["request_id"].as_str().map(str::to_string),
            retry_after_seconds,
        }
    }
    pub(crate) fn decode<T: DeserializeOwned>(response: Response) -> CompanyResult<T> {
        if !response.status().is_success() {
            return Err(Self::response_error(response));
        }
        let mut bytes = Vec::new();
        response
            .take(2_097_153)
            .read_to_end(&mut bytes)
            .map_err(|_| Self::network_error())?;
        if bytes.len() > 2_097_152 {
            return Err(CompanyError::new(
                "INVALID_RESPONSE",
                "Phản hồi Store vượt giới hạn.",
            ));
        }
        let value: Value = serde_json::from_slice(&bytes)
            .map_err(|_| CompanyError::new("INVALID_RESPONSE", "Store không trả JSON hợp lệ."))?;
        serde_json::from_value(value["data"].clone()).map_err(|_| {
            CompanyError::new("INVALID_RESPONSE", "Dữ liệu Store không đúng hợp đồng API.")
        })
    }
    pub(crate) fn network_error() -> CompanyError {
        CompanyError::new(
            "NETWORK_ERROR",
            "Không kết nối được Skill Store. Kiểm tra mạng nội bộ và thử lại.",
        )
    }
    pub fn login(&self, username: &str, password: &str) -> CompanyResult<StoredSession> {
        let response = self
            .client
            .post(self.endpoint("/auth/login")?)
            .timeout(Duration::from_secs(15))
            .json(&json!({"username":username,"password":password}))
            .send()
            .map_err(|_| Self::network_error())?;
        Self::decode(response)
    }
    pub fn logout(&self, token: &str) -> CompanyResult<()> {
        let response = self
            .client
            .post(self.endpoint("/auth/logout")?)
            .timeout(Duration::from_secs(15))
            .bearer_auth(token)
            .send()
            .map_err(|_| Self::network_error())?;
        if response.status().as_u16() == 204 {
            Ok(())
        } else {
            Err(Self::response_error(response))
        }
    }
    pub fn search(&self, token: &str, query: &str, page: u32) -> CompanyResult<SkillPage> {
        if query.chars().count() > 200 || page == 0 {
            return Err(CompanyError::new(
                "VALIDATION_ERROR",
                "Từ khóa hoặc số trang không hợp lệ.",
            ));
        }
        Self::decode(
            self.client
                .get(self.endpoint("/skills")?)
                .timeout(Duration::from_secs(15))
                .bearer_auth(token)
                .query(&[
                    ("q", query.trim()),
                    ("page", &page.to_string()),
                    ("page_size", "20"),
                ])
                .send()
                .map_err(|_| Self::network_error())?,
        )
    }
    pub fn detail(&self, token: &str, id: &str) -> CompanyResult<Skill> {
        let detail: Skill = Self::decode(
            self.client
                .get(self.endpoint(&format!("/skills/{}", urlencoding::encode(id)))?)
                .timeout(Duration::from_secs(15))
                .bearer_auth(token)
                .send()
                .map_err(|_| Self::network_error())?,
        )?;
        if detail.id != id || detail.description.is_none() {
            return Err(CompanyError::new(
                "INVALID_RESPONSE",
                "Detail không khớp định danh skill hoặc thiếu description đầy đủ.",
            ));
        }
        Ok(detail)
    }
}
