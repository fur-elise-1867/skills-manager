use super::contracts::{
    CatalogMode, Mode, Page, SkillDetail, SkillMetadata, SkillSummary, Source, Taxonomy, UserView,
};
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct WebTaxonomyData {
    pub categories: Vec<WebCategory>,
    #[serde(default)]
    pub stages: Vec<WebStage>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebCategory {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WebStage {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSkillSearchResult {
    #[serde(alias = "total")]
    pub total_count: u64,
    pub page: u32,
    pub page_size: u32,
    #[serde(default, alias = "list")]
    pub items: Vec<WebSkillItem>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSkillItem {
    #[serde(alias = "skillId")]
    pub id: serde_json::Value,
    #[serde(alias = "skillName")]
    pub name: String,
    pub description: Option<String>,
    pub creator_id: Option<serde_json::Value>,
    pub creator_name: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub categories: Vec<serde_json::Value>,
    #[serde(default)]
    pub stages: Vec<serde_json::Value>,
    #[serde(default)]
    pub hashtags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebSkillDetail {
    #[serde(alias = "skillId")]
    pub id: serde_json::Value,
    #[serde(alias = "skillName")]
    pub name: String,
    pub description: Option<String>,
    pub creator_id: Option<serde_json::Value>,
    pub creator_name: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(default)]
    pub categories: Vec<serde_json::Value>,
    #[serde(default)]
    pub stages: Vec<serde_json::Value>,
    #[serde(default)]
    pub hashtags: Vec<String>,
    pub version: Option<u64>,
}

pub fn clean_html_description(input: &str) -> String {
    let preprocessed = input
        .replace("<br>", "\n")
        .replace("<br/>", "\n")
        .replace("<br />", "\n")
        .replace("</p>", "\n");
    let mut in_tag = false;
    let mut cleaned = String::new();
    for ch in preprocessed.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => cleaned.push(ch),
            _ => {}
        }
    }
    let decoded = cleaned
        .replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'");
    decoded.trim().to_string()
}

fn extract_names(values: &[serde_json::Value]) -> Vec<String> {
    values
        .iter()
        .filter_map(|v| match v {
            serde_json::Value::String(s) => {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            serde_json::Value::Object(map) => map
                .get("name")
                .and_then(|n| n.as_str())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty()),
            _ => None,
        })
        .collect()
}

fn json_id_to_string(id: &serde_json::Value) -> String {
    match id {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        _ => String::new(),
    }
}

pub fn map_web_taxonomy(data: &WebTaxonomyData) -> Taxonomy {
    let sources = data
        .categories
        .iter()
        .map(|c| Source {
            id: c.id.to_string(),
            label: c.name.clone(),
        })
        .collect();
    Taxonomy {
        catalog_modes: vec![Mode {
            id: CatalogMode::AllTime,
            label: "All Time".into(),
            available: true,
        }],
        sources,
    }
}

pub fn map_web_search_page(web: WebSkillSearchResult) -> Page<SkillSummary> {
    let items = web
        .items
        .into_iter()
        .map(|item| {
            let id = json_id_to_string(&item.id);
            let creator_id = item
                .creator_id
                .as_ref()
                .map(json_id_to_string)
                .unwrap_or_else(|| "0".into());
            let creator_name = item.creator_name.unwrap_or_else(|| "Store User".into());
            let cat_names = extract_names(&item.categories);
            let description = clean_html_description(item.description.as_deref().unwrap_or(""));
            let updated_at = item
                .updated_at
                .or(item.created_at)
                .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
            SkillSummary {
                metadata: SkillMetadata {
                    id,
                    name: item.name,
                    description,
                    metadata_revision: 1,
                    roles: item.roles,
                    tags: item.hashtags,
                    category_ids: cat_names.clone(),
                    source_ids: cat_names,
                    creator: UserView {
                        id: creator_id,
                        display_name: creator_name,
                    },
                    compatible_agents: None,
                    can_download: false,
                    updated_at,
                },
                latest_release: None,
            }
        })
        .collect();
    Page {
        items,
        page: web.page,
        page_size: web.page_size,
        total_items: web.total_count,
    }
}

pub fn map_web_skill_detail(detail: WebSkillDetail, base_url: Option<&str>) -> SkillDetail {
    let id = json_id_to_string(&detail.id);
    let creator_id = detail
        .creator_id
        .as_ref()
        .map(json_id_to_string)
        .unwrap_or_else(|| "0".into());
    let creator_name = detail.creator_name.unwrap_or_else(|| "Store User".into());
    let cat_names = extract_names(&detail.categories);
    let description = clean_html_description(detail.description.as_deref().unwrap_or(""));
    let updated_at = detail
        .updated_at
        .or(detail.created_at)
        .unwrap_or_else(|| chrono::Utc::now().to_rfc3339());
    let store_page_url = base_url.map(|base| format!("{}/skills/{}", base.trim_end_matches('/'), id));
    SkillDetail {
        summary: SkillSummary {
            metadata: SkillMetadata {
                id,
                name: detail.name,
                description,
                metadata_revision: detail.version.unwrap_or(1),
                roles: detail.roles,
                tags: detail.hashtags,
                category_ids: cat_names.clone(),
                source_ids: cat_names,
                creator: UserView {
                    id: creator_id,
                    display_name: creator_name,
                },
                compatible_agents: None,
                can_download: false,
                updated_at,
            },
            latest_release: None,
        },
        store_page_url,
        latest_release: None,
    }
}

pub fn map_source_id_to_category_id(source_id: &str) -> String {
    let trimmed = source_id.trim();
    if trimmed.chars().all(|c| c.is_ascii_digit()) && !trimmed.is_empty() {
        return trimmed.to_string();
    }
    match trimmed.to_lowercase().as_str() {
        "quality" => "1".to_string(),
        "security" => "2".to_string(),
        "devops" => "3".to_string(),
        "ai/ml" | "ai" | "ml" => "4".to_string(),
        "documentation" => "5".to_string(),
        "other" => "6".to_string(),
        _ => trimmed.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_map_source_id_to_category_id() {
        assert_eq!(map_source_id_to_category_id("1"), "1");
        assert_eq!(map_source_id_to_category_id("2"), "2");
        assert_eq!(map_source_id_to_category_id("Quality"), "1");
        assert_eq!(map_source_id_to_category_id("quality"), "1");
        assert_eq!(map_source_id_to_category_id("Security"), "2");
        assert_eq!(map_source_id_to_category_id("DevOps"), "3");
        assert_eq!(map_source_id_to_category_id("AI/ML"), "4");
        assert_eq!(map_source_id_to_category_id("unknown"), "unknown");
    }

    #[test]
    fn test_clean_html_description() {
        assert_eq!(
            clean_html_description("<p>Summarize sprint progress and blockers.</p>"),
            "Summarize sprint progress and blockers."
        );
        assert_eq!(
            clean_html_description("<p>Line 1<br/>Line 2&nbsp;&amp;&nbsp;Line 3</p>"),
            "Line 1\nLine 2 & Line 3"
        );
        assert_eq!(clean_html_description(""), "");
        assert_eq!(
            clean_html_description("Safe text without HTML"),
            "Safe text without HTML"
        );
    }

    #[test]
    fn test_map_web_taxonomy() {
        let raw = include_str!("../../../contracts/store-web/v1/fixtures/taxonomy.json");
        let envelope: serde_json::Value = serde_json::from_str(raw).unwrap();
        let data: WebTaxonomyData = serde_json::from_value(envelope["data"].clone()).unwrap();
        let taxonomy = map_web_taxonomy(&data);
        assert_eq!(taxonomy.sources.len(), 6);
        assert_eq!(taxonomy.sources[0].id, "1");
        assert_eq!(taxonomy.sources[0].label, "Quality");
        assert_eq!(taxonomy.catalog_modes.len(), 1);
        assert_eq!(taxonomy.catalog_modes[0].id, CatalogMode::AllTime);
    }

    #[test]
    fn test_map_web_search_page() {
        let raw = include_str!("../../../contracts/store-web/v1/fixtures/catalog.json");
        let envelope: serde_json::Value = serde_json::from_str(raw).unwrap();
        let data: WebSkillSearchResult = serde_json::from_value(envelope["data"].clone()).unwrap();
        let page = map_web_search_page(data);
        assert_eq!(page.total_items, 1);
        assert_eq!(page.page, 1);
        assert_eq!(page.items.len(), 1);
        let item = &page.items[0];
        assert_eq!(item.metadata.id, "35");
        assert_eq!(item.metadata.name, "Jira Weekly Report");
        assert_eq!(
            item.metadata.description,
            "Summarize sprint progress and blockers."
        );
        assert_eq!(item.metadata.creator.display_name, "Admin");
        assert!(!item.metadata.can_download);
        assert!(item.latest_release.is_none());
    }

    #[test]
    fn test_map_web_skill_detail() {
        let raw = include_str!("../../../contracts/store-web/v1/fixtures/detail.json");
        let envelope: serde_json::Value = serde_json::from_str(raw).unwrap();
        let data: WebSkillDetail = serde_json::from_value(envelope["data"].clone()).unwrap();
        let detail = map_web_skill_detail(data, Some("http://localhost:8080/skm"));
        assert_eq!(detail.summary.metadata.id, "35");
        assert_eq!(detail.summary.metadata.name, "Jira Weekly Report");
        assert_eq!(
            detail.store_page_url,
            Some("http://localhost:8080/skm/skills/35".into())
        );
        assert!(!detail.summary.metadata.can_download);
        assert!(detail.latest_release.is_none());
    }
}
