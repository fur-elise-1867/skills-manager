use super::types::{McpReport, McpState};
use serde_json::Value;
use std::{fs, path::Path};

#[derive(Debug, Clone)]
pub enum McpRequirements {
    NoneDeclared,
    NoRequirements,
    Names(Vec<String>),
    Invalid,
}

pub fn parse_requirements(description: &str) -> McpRequirements {
    let declarations: Vec<_> = description
        .lines()
        .filter_map(|line| line.trim().strip_prefix("MCP bắt buộc:"))
        .collect();
    if declarations.is_empty() {
        return McpRequirements::NoneDeclared;
    }
    if declarations.len() != 1 {
        return McpRequirements::Invalid;
    }
    let declaration = declarations[0].trim();
    if declaration == "không" {
        return McpRequirements::NoRequirements;
    }
    let mut names = Vec::new();
    for name in declaration.split(',').map(str::trim) {
        if name.is_empty() || name == "không" {
            return McpRequirements::Invalid;
        }
        if !names.iter().any(|existing| existing == name) {
            names.push(name.to_string());
        }
    }
    McpRequirements::Names(names)
}

fn report(state: McpState, required: Vec<String>, reason: Option<&str>) -> McpReport {
    McpReport {
        state,
        required,
        present: Vec::new(),
        missing: Vec::new(),
        reason: reason.map(str::to_string),
        checked_at: chrono::Utc::now().to_rfc3339(),
    }
}

pub fn check_config(requirements: &McpRequirements, bytes: &[u8]) -> McpReport {
    let names = match requirements {
        McpRequirements::NoneDeclared => return report(McpState::Undeclared, vec![], None),
        McpRequirements::NoRequirements => return report(McpState::NotRequired, vec![], None),
        McpRequirements::Invalid => {
            return report(
                McpState::Unknown,
                vec![],
                Some("Description khai báo MCP không đúng quy ước."),
            )
        }
        McpRequirements::Names(names) => names.clone(),
    };
    let bytes = bytes.strip_prefix(&[0xef, 0xbb, 0xbf]).unwrap_or(bytes);
    let config: Value = match serde_json::from_slice(bytes) {
        Ok(value) => value,
        Err(_) => {
            return report(
                McpState::Unknown,
                names,
                Some("mcp.json không phải JSON hợp lệ."),
            )
        }
    };
    let servers = match config
        .as_object()
        .and_then(|root| root.get("mcpServers"))
        .and_then(Value::as_object)
    {
        Some(servers) => servers,
        None => {
            return report(
                McpState::Unknown,
                names,
                Some("mcpServers phải là một object trong mcp.json."),
            )
        }
    };
    let mut result = report(McpState::Present, names.clone(), None);
    for name in names {
        if servers.contains_key(&name) {
            result.present.push(name);
        } else {
            result.missing.push(name);
        }
    }
    if !result.missing.is_empty() {
        result.state = McpState::Missing;
    }
    result
}

pub fn check_global_mcp(requirements: &McpRequirements, profile: &Path) -> McpReport {
    if !matches!(requirements, McpRequirements::Names(_)) {
        return check_config(requirements, b"{}");
    }
    match fs::read(profile.join(".aipro/mcp.json")) {
        Ok(bytes) => check_config(requirements, &bytes),
        Err(_) => {
            let names = match requirements {
                McpRequirements::Names(names) => names.clone(),
                _ => vec![],
            };
            report(
                McpState::Unknown,
                names,
                Some("Không đọc được file MCP Global. Kiểm tra đường dẫn và quyền đọc mcp.json."),
            )
        }
    }
}
