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
pub struct DownloadSink<'a> {
    pub file: &'a mut tokio::fs::File,
    pub expected: &'a super::contracts::PackageInfo,
    pub limit: u64,
    pub cancel: &'a std::sync::atomic::AtomicBool,
}
pub trait HttpTransport: Send + Sync {
    fn send(&self, request: HttpRequest) -> HttpFuture<'_>;
    fn download<'a>(
        &'a self,
        _request: HttpRequest,
        _sink: &'a mut DownloadSink<'_>,
    ) -> HttpFuture<'a> {
        Box::pin(async {
            Err(SdsError::new(
                "INVALID_RESPONSE",
                "Package streaming is unavailable",
                false,
            ))
        })
    }
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
    #[cfg(debug_assertions)]
    pub fn fixture() -> Result<Self, SdsError> {
        // Synthetic fixture traffic never goes through an employee's proxy.
        let client = reqwest::Client::builder()
            .no_proxy()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|_| {
                SdsError::new(
                    "STORE_UNAVAILABLE",
                    "Development fixture transport is unavailable",
                    false,
                )
            })?;
        Ok(Self { client })
    }
}
impl HttpTransport for ReqwestTransport {
    fn download<'a>(
        &'a self,
        request: HttpRequest,
        sink: &'a mut DownloadSink<'_>,
    ) -> HttpFuture<'a> {
        Box::pin(async move {
            use sha2::{Digest, Sha256};
            use tokio::io::{AsyncSeekExt, AsyncWriteExt};
            let failure =
                || SdsError::new("STORE_UNAVAILABLE", "Skills Store is unavailable", true);
            let mut builder = self.client.request(request.method, request.url);
            if let Some(token) = request.bearer {
                builder = builder.bearer_auth(token);
            }
            let mut response = tokio::select! {
                result = builder.send() => result.map_err(|_| failure())?,
                _ = wait_download_cancel(sink.cancel) => return Err(download_cancelled()),
            };
            let status = response.status().as_u16();
            if status != 200 {
                if (200..300).contains(&status) {
                    return Err(catalog_response_error());
                }
                return Ok(HttpResponse {
                    status,
                    body: vec![],
                });
            }
            if response
                .content_length()
                .is_some_and(|length| length > sink.limit)
            {
                return Err(download_limit());
            }
            if response
                .content_length()
                .is_some_and(|length| length != sink.expected.size_bytes)
            {
                return Err(download_integrity());
            }
            sink.file.set_len(0).await.map_err(|_| download_io())?;
            sink.file
                .seek(std::io::SeekFrom::Start(0))
                .await
                .map_err(|_| download_io())?;
            let mut count = 0u64;
            let mut hash = Sha256::new();
            loop {
                let chunk = tokio::select! {
                    result = response.chunk() => result.map_err(|_| failure())?,
                    _ = wait_download_cancel(sink.cancel) => return Err(download_cancelled()),
                };
                let Some(chunk) = chunk else {
                    break;
                };
                count = count
                    .checked_add(chunk.len() as u64)
                    .ok_or_else(download_limit)?;
                if count > sink.limit {
                    return Err(download_limit());
                }
                if count > sink.expected.size_bytes {
                    return Err(download_integrity());
                }
                hash.update(&chunk);
                sink.file
                    .write_all(&chunk)
                    .await
                    .map_err(|_| download_io())?;
            }
            if count != sink.expected.size_bytes
                || hex::encode(hash.finalize()) != sink.expected.sha256.to_ascii_lowercase()
            {
                return Err(download_integrity());
            }
            sink.file.flush().await.map_err(|_| download_io())?;
            sink.file.sync_all().await.map_err(|_| download_io())?;
            Ok(HttpResponse {
                status,
                body: vec![],
            })
        })
    }
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
    let base_path = base.path().trim_end_matches('/');
    let desktop_prefix = format!("{}/api/v1/desktop/", base_path);
    let skills_prefix = format!("{}/api/v1/skills", base_path);
    let url_path = url.path();
    let is_desktop = url_path.starts_with(&desktop_prefix);
    let is_skills = url_path == skills_prefix || url_path.starts_with(&format!("{}/", skills_prefix));
    if base.origin() != url.origin()
        || !(is_desktop || is_skills)
        || unsafe_path(url_path)
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
    CatalogQuery, Page, Release, SkillDetail, SkillMetadata, SkillSummary, Taxonomy, StoreProfile,
};
use super::session::SessionService;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApiMode {
    Auto,
    Desktop,
    Web,
}

pub struct StoreClient {
    session: Arc<SessionService>,
    api_mode: std::sync::atomic::AtomicU8,
    mode_generation: std::sync::atomic::AtomicU64,
}
impl StoreClient {
    pub fn new(session: Arc<SessionService>) -> Self {
        Self {
            session,
            api_mode: std::sync::atomic::AtomicU8::new(0),
            mode_generation: std::sync::atomic::AtomicU64::new(0),
        }
    }
    #[cfg(feature = "sds")]
    pub async fn check_updates(
        &self,
        items: &[super::updates_metadata::KnownRelease],
    ) -> Result<super::updates_metadata::UpdateCheck, SdsError> {
        use super::updates_metadata::{invalid_response, KnownRelease, UpdateCheck, UpdateStatus};
        use std::collections::{HashMap, HashSet};
        // Capture one account/profile generation for the entire chunked request.
        // A response from an earlier account cannot become authoritative metadata.
        let mut seen = HashSet::new();
        let mut unique = Vec::new();
        for item in items {
            if [&item.skill_id, &item.release_id].iter().any(|id| {
                id.is_empty() || id.len() > 256 || id.chars().any(char::is_control)
            }) {
                return Err(SdsError::new("INVALID_REQUEST", "Invalid release identifier", false));
            }
            if seen.insert(item.clone()) { unique.push(item.clone()); }
        }
        let mut result = UpdateCheck { checked_at: chrono::Utc::now().to_rfc3339(), items: Vec::new() };
        if unique.is_empty() { return Ok(result); }
        let (profile, context) = self.session.scope().await?;
        let url = join_api(&profile.base_url, "api/v1/desktop/updates/check")?;
        for chunk in unique.chunks(100) {
            let response = self.session.request_for(&context, HttpRequest::post(url.clone(),
                serde_json::json!({"items": chunk}))).await?;
            #[derive(Deserialize)]
            struct Envelope { success: bool, data: UpdateCheck }
            let envelope: Envelope = serde_json::from_slice(&response.body).map_err(|_| invalid_response())?;
            let check = envelope.data;
            if !envelope.success || chrono::DateTime::parse_from_rfc3339(&check.checked_at).is_err()
                || check.items.len() != chunk.len() {
                return Err(invalid_response());
            }
            let expected: HashSet<_> = chunk.iter().cloned().collect();
            let mut by_pair = HashMap::new();
            for item in check.items {
                let pair = KnownRelease { skill_id: item.skill_id.clone(), release_id: item.current_release_id.clone() };
                if !expected.contains(&pair) || by_pair.contains_key(&pair) { return Err(invalid_response()); }
                match item.status {
                    UpdateStatus::UpdateAvailable | UpdateStatus::UpToDate => {
                        let latest = item.latest_release.as_ref().ok_or_else(invalid_response)?;
                        if latest.id.is_empty() || latest.id.len() > 256 || latest.id.chars().any(char::is_control)
                            || latest.sequence == 0 || latest.version.trim().is_empty()
                            || (item.status == UpdateStatus::UpdateAvailable && latest.id == item.current_release_id) {
                            return Err(invalid_response());
                        }
                    }
                    UpdateStatus::Unavailable | UpdateStatus::UnknownRelease => {
                        if item.latest_release.is_some() { return Err(invalid_response()); }
                    }
                }
                by_pair.insert(pair, item);
            }
            for pair in chunk { result.items.push(by_pair.remove(pair).ok_or_else(invalid_response)?); }
            result.checked_at = check.checked_at;
        }
        self.session.with_current_context(&context, || Ok(result)).await
    }
    pub async fn download(
        &self,
        release: &Release,
        dest: &std::path::Path,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<(), SdsError> {
        let (_, context) = self.session.scope().await?;
        self.download_for(&context, release, dest, cancel).await
    }
    pub async fn download_for(
        &self,
        context: &super::contracts::SessionContext,
        release: &Release,
        dest: &std::path::Path,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<(), SdsError> {
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            return Err(download_cancelled());
        }
        const LIMIT: u64 = 104_857_600;
        if release.package.size_bytes > LIMIT {
            return Err(download_limit());
        }
        if release.package.sha256.len() != 64
            || !release
                .package
                .sha256
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        {
            return Err(download_integrity());
        }
        let (profile, _) = self.session.scope().await?;
        let url = join_api(&profile.base_url, &release.package.download_path)?;
        validate_protected_url(&profile.base_url, &url)?;
        let parent = dest.parent().ok_or_else(download_io)?;
        super::targets::validate_directory_path(parent)?;
        if std::fs::symlink_metadata(dest).is_ok() {
            return Err(download_io());
        }
        let temporary = tempfile::Builder::new()
            .prefix(".sds-download-")
            .tempfile_in(parent)
            .map_err(|_| download_io())?;
        let (file, path) = temporary.into_parts();
        let mut file = tokio::fs::File::from_std(file);
        let mut sink = DownloadSink {
            file: &mut file,
            expected: &release.package,
            limit: LIMIT,
            cancel,
        };
        self.session
            .download_for(context, HttpRequest::get(url), &mut sink)
            .await?;
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            return Err(download_cancelled());
        }
        drop(file);
        self.session
            .with_current_context(context, || {
                if cancel.load(std::sync::atomic::Ordering::Acquire) {
                    return Err(download_cancelled());
                }
                path.persist_noclobber(dest).map_err(|_| download_io())?;
                Ok(())
            })
            .await
    }
    fn current_mode(&self, generation: u64) -> ApiMode {
        if self.mode_generation.load(std::sync::atomic::Ordering::Acquire) != generation {
            return ApiMode::Auto;
        }
        match self.api_mode.load(std::sync::atomic::Ordering::Acquire) {
            1 => ApiMode::Desktop,
            2 => ApiMode::Web,
            _ => ApiMode::Auto,
        }
    }
    fn set_mode(&self, generation: u64, mode: ApiMode) {
        let val = match mode {
            ApiMode::Auto => 0,
            ApiMode::Desktop => 1,
            ApiMode::Web => 2,
        };
        self.api_mode.store(val, std::sync::atomic::Ordering::Release);
        self.mode_generation.store(generation, std::sync::atomic::Ordering::Release);
    }
    pub async fn taxonomy(&self) -> Result<Taxonomy, SdsError> {
        let (profile, context) = self.session.scope().await?;
        let mode = self.current_mode(context.generation);
        if mode == ApiMode::Web {
            return self.web_taxonomy(&profile, &context).await;
        }
        if mode == ApiMode::Desktop {
            return self.get_for(&context, &["taxonomy"], None).await;
        }
        let desktop_url = join_api(&profile.base_url, "api/v1/desktop/taxonomy")?;
        let response = self
            .session
            .request_raw_for(&context, HttpRequest::get(desktop_url))
            .await?;
        if (200..300).contains(&response.status) {
            #[derive(Deserialize)]
            struct Envelope {
                success: bool,
                data: Taxonomy,
            }
            let envelope: Envelope =
                serde_json::from_slice(&response.body).map_err(|_| catalog_response_error())?;
            if !envelope.success {
                return Err(catalog_response_error());
            }
            self.set_mode(context.generation, ApiMode::Desktop);
            return Ok(envelope.data);
        }
        let web_result = self.web_taxonomy(&profile, &context).await;
        if web_result.is_ok() {
            self.set_mode(context.generation, ApiMode::Web);
        } else if response.status >= 500 {
            self.session.mark_offline_for(&context).await;
        }
        web_result
    }
    async fn web_taxonomy(
        &self,
        profile: &StoreProfile,
        context: &super::contracts::SessionContext,
    ) -> Result<Taxonomy, SdsError> {
        let url = join_api(&profile.base_url, "api/v1/skills/taxonomy")?;
        let response = self
            .session
            .request_raw_for(context, HttpRequest::get(url))
            .await?;
        if !(200..300).contains(&response.status) {
            if response.status >= 500 {
                self.session.mark_offline_for(context).await;
            }
            return Err(super::session::http_error(response.status, false));
        }
        #[derive(Deserialize)]
        struct Envelope {
            success: bool,
            data: super::store_web::WebTaxonomyData,
        }
        let envelope: Envelope =
            serde_json::from_slice(&response.body).map_err(|_| catalog_response_error())?;
        if !envelope.success {
            return Err(catalog_response_error());
        }
        Ok(super::store_web::map_web_taxonomy(&envelope.data))
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
        let (profile, context) = self.session.scope().await?;
        let mode = self.current_mode(context.generation);
        if mode == ApiMode::Web {
            return self.web_search(&profile, &context, &query).await;
        }
        if mode == ApiMode::Desktop {
            let page: Page<SkillSummary> = self.get_for(&context, &["skills"], Some(&query)).await?;
            self.validate_page(&page, &query)?;
            return Ok(page);
        }
        let desktop_url = self.desktop_search_url(&profile, &query)?;
        let response = self
            .session
            .request_raw_for(&context, HttpRequest::get(desktop_url))
            .await?;
        if (200..300).contains(&response.status) {
            #[derive(Deserialize)]
            struct Envelope {
                success: bool,
                data: Page<SkillSummary>,
            }
            let envelope: Envelope =
                serde_json::from_slice(&response.body).map_err(|_| catalog_response_error())?;
            if !envelope.success {
                return Err(catalog_response_error());
            }
            let page = envelope.data;
            self.validate_page(&page, &query)?;
            self.set_mode(context.generation, ApiMode::Desktop);
            return Ok(page);
        }
        let web_result = self.web_search(&profile, &context, &query).await;
        if web_result.is_ok() {
            self.set_mode(context.generation, ApiMode::Web);
        } else if response.status >= 500 {
            self.session.mark_offline_for(&context).await;
        }
        web_result
    }
    fn desktop_search_url(&self, profile: &StoreProfile, query: &CatalogQuery) -> Result<Url, SdsError> {
        let mut url = join_api(&profile.base_url, "api/v1/desktop/skills")?;
        {
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
        Ok(url)
    }
    async fn web_search(
        &self,
        profile: &StoreProfile,
        context: &super::contracts::SessionContext,
        query: &CatalogQuery,
    ) -> Result<Page<SkillSummary>, SdsError> {
        let mut url = join_api(&profile.base_url, "api/v1/skills")?;
        {
            let mut pairs = url.query_pairs_mut();
            pairs
                .append_pair("page", &query.page.to_string())
                .append_pair("pageSize", &query.page_size.to_string());
            if !query.q.is_empty() {
                pairs.append_pair("skillName", &query.q);
            }
            if let Some(source) = &query.source_id {
                let cat_id = super::store_web::map_source_id_to_category_id(source);
                pairs.append_pair("categoryIds", &cat_id);
            }
        }
        let response = self
            .session
            .request_raw_for(context, HttpRequest::get(url))
            .await?;
        if !(200..300).contains(&response.status) {
            if response.status >= 500 {
                self.session.mark_offline_for(context).await;
            }
            return Err(super::session::http_error(response.status, false));
        }
        #[derive(Deserialize)]
        struct Envelope {
            success: bool,
            data: super::store_web::WebSkillSearchResult,
        }
        let envelope: Envelope =
            serde_json::from_slice(&response.body).map_err(|_| catalog_response_error())?;
        if !envelope.success {
            return Err(catalog_response_error());
        }
        let page = super::store_web::map_web_search_page(envelope.data);
        self.validate_page(&page, query)?;
        Ok(page)
    }
    fn validate_page(&self, page: &Page<SkillSummary>, query: &CatalogQuery) -> Result<(), SdsError> {
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
        Ok(())
    }
    pub async fn detail(&self, skill_id: &str) -> Result<SkillDetail, SdsError> {
        let (_, context) = self.session.scope().await?;
        self.detail_for(&context, skill_id).await
    }
    pub async fn detail_for(
        &self,
        context: &super::contracts::SessionContext,
        skill_id: &str,
    ) -> Result<SkillDetail, SdsError> {
        if skill_id.is_empty()
            || skill_id.len() > 256
            || skill_id.contains(['/', '\\'])
            || skill_id.chars().any(char::is_control)
        {
            return Err(SdsError::new(
                "INVALID_REQUEST",
                "Invalid Store content identifier",
                false,
            ));
        }
        let (profile, _) = self.session.scope().await?;
        let mode = self.current_mode(context.generation);
        if mode == ApiMode::Web {
            return self.web_detail(&profile, context, skill_id).await;
        }
        if mode == ApiMode::Desktop {
            return self.desktop_detail(context, skill_id).await;
        }
        let desktop_url = join_api(&profile.base_url, &format!("api/v1/desktop/skills/{}", skill_id))?;
        let response = self
            .session
            .request_raw_for(context, HttpRequest::get(desktop_url))
            .await?;
        if (200..300).contains(&response.status) {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct WireDetail {
                #[serde(flatten)]
                metadata: SkillMetadata,
                store_page_url: Option<String>,
                latest_release: Option<Release>,
            }
            #[derive(Deserialize)]
            struct Envelope {
                success: bool,
                data: WireDetail,
            }
            let envelope: Envelope =
                serde_json::from_slice(&response.body).map_err(|_| catalog_response_error())?;
            if !envelope.success {
                return Err(catalog_response_error());
            }
            let detail = envelope.data;
            if detail.metadata.id != skill_id
                || detail.metadata.name.trim().is_empty()
                || detail
                    .latest_release
                    .as_ref()
                    .is_some_and(|release| release.skill_id != skill_id)
            {
                return Err(catalog_response_error());
            }
            self.set_mode(context.generation, ApiMode::Desktop);
            return Ok(SkillDetail {
                summary: SkillSummary {
                    metadata: detail.metadata,
                    latest_release: detail.latest_release.as_ref().map(Release::reference),
                },
                store_page_url: detail.store_page_url,
                latest_release: detail.latest_release,
            });
        }
        let web_result = self.web_detail(&profile, context, skill_id).await;
        if web_result.is_ok() {
            self.set_mode(context.generation, ApiMode::Web);
        } else if response.status >= 500 {
            self.session.mark_offline_for(context).await;
        }
        web_result
    }
    async fn desktop_detail(
        &self,
        context: &super::contracts::SessionContext,
        skill_id: &str,
    ) -> Result<SkillDetail, SdsError> {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct WireDetail {
            #[serde(flatten)]
            metadata: SkillMetadata,
            store_page_url: Option<String>,
            latest_release: Option<Release>,
        }
        let detail: WireDetail = self.get_for(context, &["skills", skill_id], None).await?;
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
    async fn web_detail(
        &self,
        profile: &StoreProfile,
        context: &super::contracts::SessionContext,
        skill_id: &str,
    ) -> Result<SkillDetail, SdsError> {
        let url = join_api(&profile.base_url, &format!("api/v1/skills/{}", skill_id))?;
        let response = self
            .session
            .request_raw_for(context, HttpRequest::get(url))
            .await?;
        if !(200..300).contains(&response.status) {
            if response.status >= 500 {
                self.session.mark_offline_for(context).await;
            }
            return Err(super::session::http_error(response.status, false));
        }
        #[derive(Deserialize)]
        struct Envelope {
            success: bool,
            data: super::store_web::WebSkillDetail,
        }
        let envelope: Envelope =
            serde_json::from_slice(&response.body).map_err(|_| catalog_response_error())?;
        if !envelope.success {
            return Err(catalog_response_error());
        }
        let detail = super::store_web::map_web_skill_detail(envelope.data, Some(&profile.base_url));
        if detail.summary.metadata.id != skill_id || detail.summary.metadata.name.trim().is_empty() {
            return Err(catalog_response_error());
        }
        Ok(detail)
    }
    pub async fn release(&self, skill_id: &str, release_id: &str) -> Result<Release, SdsError> {
        let (_, context) = self.session.scope().await?;
        self.release_for(&context, skill_id, release_id).await
    }
    pub async fn release_for(
        &self,
        context: &super::contracts::SessionContext,
        skill_id: &str,
        release_id: &str,
    ) -> Result<Release, SdsError> {
        let release: Release = self
            .get_for(context, &["skills", skill_id, "releases", release_id], None)
            .await?;
        if release.skill_id != skill_id || release.id != release_id {
            return Err(catalog_response_error());
        }
        Ok(release)
    }
    async fn get_for<T: serde::de::DeserializeOwned>(
        &self,
        context: &super::contracts::SessionContext,
        segments: &[&str],
        query: Option<&CatalogQuery>,
    ) -> Result<T, SdsError> {
        let (profile, _) = self.session.scope().await?;
        self.session
            .with_current_context(context, || Ok(()))
            .await?;
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
            .request_for(context, HttpRequest::get(url))
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

async fn wait_download_cancel(cancel: &std::sync::atomic::AtomicBool) {
    while !cancel.load(std::sync::atomic::Ordering::Acquire) {
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}
fn download_cancelled() -> SdsError {
    SdsError::new("CANCELLED", "Package download was cancelled", false)
}
fn download_integrity() -> SdsError {
    SdsError::new(
        "PACKAGE_INTEGRITY",
        "The package size or SHA-256 does not match its release. Download it again.",
        false,
    )
}
fn download_limit() -> SdsError {
    SdsError::new(
        "PACKAGE_LIMIT",
        "The package exceeds the 100 MiB download limit",
        false,
    )
}
fn download_io() -> SdsError {
    SdsError::new(
        "PACKAGE_IO",
        "The package staging file could not be read or saved",
        false,
    )
}
