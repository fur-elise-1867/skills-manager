use super::error::SdsError;
use super::profile::Environment;
use reqwest::Url;
use std::{future::Future, pin::Pin, time::Duration};

#[derive(Clone)]
pub struct HttpRequest {
    pub method: reqwest::Method,
    pub url: Url,
    pub body: Option<serde_json::Value>,
    pub(crate) bearer: Option<String>,
}
impl HttpRequest {
    pub fn get(url: Url) -> Self {
        Self {
            method: reqwest::Method::GET,
            url,
            body: None,
            bearer: None,
        }
    }
    pub fn post(url: Url, body: serde_json::Value) -> Self {
        Self {
            method: reqwest::Method::POST,
            url,
            body: Some(body),
            bearer: None,
        }
    }
}
pub struct HttpResponse {
    pub status: u16,
    pub body: Vec<u8>,
}
pub type HttpFuture<'a> = Pin<Box<dyn Future<Output = Result<HttpResponse, SdsError>> + Send + 'a>>;
pub trait HttpTransport: Send + Sync {
    fn send(&self, request: HttpRequest) -> HttpFuture<'_>;
}
pub struct ReqwestTransport {
    client: reqwest::Client,
}
impl ReqwestTransport {
    pub fn new() -> Result<Self, SdsError> {
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| {
                SdsError::new(
                    "STORE_UNAVAILABLE",
                    "Store transport could not be initialized",
                    true,
                )
            })?;
        Ok(Self { client })
    }
}
impl HttpTransport for ReqwestTransport {
    fn send(&self, request: HttpRequest) -> HttpFuture<'_> {
        Box::pin(async move {
            let failure =
                || SdsError::new("STORE_UNAVAILABLE", "Skills Store is unavailable", true);
            let mut builder = self.client.request(request.method, request.url);
            if let Some(token) = request.bearer {
                builder = builder.bearer_auth(token);
            }
            if let Some(body) = request.body {
                builder = builder.json(&body);
            }
            let mut response = builder.send().await.map_err(|_| failure())?;
            let status = response.status().as_u16();
            let mut body = Vec::new();
            while let Some(chunk) = response.chunk().await.map_err(|_| failure())? {
                if body.len() + chunk.len() > 2 * 1024 * 1024 {
                    return Err(SdsError::new(
                        "INVALID_RESPONSE",
                        "Store JSON response exceeds the supported limit",
                        false,
                    ));
                }
                body.extend_from_slice(&chunk);
            }
            Ok(HttpResponse { status, body })
        })
    }
}

pub fn canonical_base_url(base: &str, environment: Environment) -> Result<String, SdsError> {
    if unsafe_path(base) {
        return Err(SdsError::invalid("Invalid Store URL"));
    }
    let url = Url::parse(base.trim()).map_err(|_| SdsError::invalid("Invalid Store URL"))?;
    let host = url
        .host_str()
        .ok_or_else(|| SdsError::invalid("Store URL requires a host"))?;
    let loopback = host == "localhost"
        || host
            .trim_matches(['[', ']'])
            .parse::<std::net::IpAddr>()
            .is_ok_and(|ip| ip.is_loopback());
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || !(url.scheme() == "https"
            || (url.scheme() == "http" && environment == Environment::Development && loopback))
    {
        return Err(SdsError::invalid(
            "Use verified HTTPS or development loopback HTTP without URL credentials",
        ));
    }
    Ok(url.as_str().trim_end_matches('/').to_string())
}

pub fn join_api(base: &str, route: &str) -> Result<Url, SdsError> {
    let route = route.strip_prefix('/').unwrap_or(route);
    if !route.starts_with("api/v1/") || unsafe_path(route) || route.contains(['?', '#', ':']) {
        return Err(SdsError::invalid("Invalid Store API route"));
    }
    Url::parse(&format!("{}/", base.trim_end_matches('/')))
        .and_then(|base| base.join(route))
        .map_err(|_| SdsError::invalid("Invalid Store URL"))
}

fn unsafe_path(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    value.contains('\\')
        || value.chars().any(char::is_control)
        || value
            .split('/')
            .any(|segment| segment == "." || segment == "..")
        || ["%2e", "%2f", "%5c", "%25"]
            .iter()
            .any(|pattern| lower.contains(pattern))
}

pub fn validate_protected_url(base: &str, url: &Url) -> Result<(), SdsError> {
    let base = Url::parse(base).map_err(|_| SdsError::invalid("Invalid Store URL"))?;
    let prefix = format!("{}/api/v1/desktop/", base.path().trim_end_matches('/'));
    if base.origin() != url.origin()
        || !url.path().starts_with(&prefix)
        || unsafe_path(url.path())
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(SdsError::invalid(
            "Protected request must stay under the configured desktop API",
        ));
    }
    Ok(())
}

use super::contracts::{
    CatalogQuery, Page, Release, SkillDetail, SkillMetadata, SkillSummary, Taxonomy,
};
use super::session::SessionService;
use serde::Deserialize;
use std::sync::Arc;

pub struct StoreClient {
    session: Arc<SessionService>,
}
impl StoreClient {
    pub fn new(session: Arc<SessionService>) -> Self {
        Self { session }
    }
    pub async fn taxonomy(&self) -> Result<Taxonomy, SdsError> {
        self.get(&["taxonomy"], None).await
    }
    pub async fn search(&self, mut query: CatalogQuery) -> Result<Page<SkillSummary>, SdsError> {
        query.q = query.q.trim().to_string();
        query.source_id = query
            .source_id
            .map(|source| source.trim().to_string())
            .filter(|source| !source.is_empty());
        if query.page == 0
            || !(1..=100).contains(&query.page_size)
            || query.q.chars().count() > 1024
            || query
                .source_id
                .as_ref()
                .is_some_and(|source| source.chars().count() > 256)
        {
            return Err(SdsError::new(
                "INVALID_REQUEST",
                "Invalid catalog filters or pagination",
                false,
            ));
        }
        let page: Page<SkillSummary> = self.get(&["skills"], Some(&query)).await?;
        if page.page != query.page
            || page.page_size != query.page_size
            || page.items.len() > query.page_size as usize
            || page.total_items < page.items.len() as u64
            || page
                .items
                .iter()
                .any(|skill| skill.metadata.id.is_empty() || skill.metadata.name.trim().is_empty())
        {
            return Err(catalog_response_error());
        }
        Ok(page)
    }
    pub async fn detail(&self, skill_id: &str) -> Result<SkillDetail, SdsError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct WireDetail {
            #[serde(flatten)]
            metadata: SkillMetadata,
            store_page_url: Option<String>,
            latest_release: Option<Release>,
        }
        let detail: WireDetail = self.get(&["skills", skill_id], None).await?;
        if detail.metadata.id != skill_id
            || detail.metadata.name.trim().is_empty()
            || detail
                .latest_release
                .as_ref()
                .is_some_and(|release| release.skill_id != skill_id)
        {
            return Err(catalog_response_error());
        }
        Ok(SkillDetail {
            summary: SkillSummary {
                metadata: detail.metadata,
                latest_release: detail.latest_release.as_ref().map(Release::reference),
            },
            store_page_url: detail.store_page_url,
            latest_release: detail.latest_release,
        })
    }
    pub async fn release(&self, skill_id: &str, release_id: &str) -> Result<Release, SdsError> {
        let release: Release = self
            .get(&["skills", skill_id, "releases", release_id], None)
            .await?;
        if release.skill_id != skill_id || release.id != release_id {
            return Err(catalog_response_error());
        }
        Ok(release)
    }
    async fn get<T: serde::de::DeserializeOwned>(
        &self,
        segments: &[&str],
        query: Option<&CatalogQuery>,
    ) -> Result<T, SdsError> {
        let (profile, context) = self.session.scope().await?;
        let mut url = join_api(&profile.base_url, "api/v1/desktop/")?;
        {
            let mut path = url
                .path_segments_mut()
                .map_err(|_| catalog_response_error())?;
            path.pop_if_empty();
            for segment in segments {
                if segment.is_empty()
                    || *segment == "."
                    || *segment == ".."
                    || segment.contains(['/', '\\'])
                    || segment.chars().any(char::is_control)
                    || segment.len() > 256
                {
                    return Err(SdsError::new(
                        "INVALID_REQUEST",
                        "Invalid Store content identifier",
                        false,
                    ));
                }
                path.push(segment);
            }
        }
        if let Some(query) = query {
            let mut pairs = url.query_pairs_mut();
            pairs
                .append_pair("q", &query.q)
                .append_pair("mode", query.mode.as_str());
            if let Some(source) = &query.source_id {
                pairs.append_pair("sourceId", source);
            }
            pairs
                .append_pair("page", &query.page.to_string())
                .append_pair("pageSize", &query.page_size.to_string());
        }
        let response = self
            .session
            .request_for(&context, HttpRequest::get(url))
            .await?;
        #[derive(Deserialize)]
        struct Envelope<T> {
            success: bool,
            data: T,
        }
        let envelope: Envelope<T> =
            serde_json::from_slice(&response.body).map_err(|_| catalog_response_error())?;
        if !envelope.success {
            return Err(catalog_response_error());
        }
        Ok(envelope.data)
    }
}
fn catalog_response_error() -> SdsError {
    SdsError::new(
        "INVALID_RESPONSE",
        "Skills Store returned an unsupported response",
        false,
    )
}
