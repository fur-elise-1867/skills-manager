use super::contracts::{McpReport, McpState};
use std::path::Path;

pub fn inspect_mcp(path: &Path, required: Option<&[String]>) -> McpReport {
    let Some(required) = required else {
        return report(
            McpState::Unknown,
            vec![],
            Some("This release has not declared MCP requirements"),
        );
    };
    if required.is_empty() {
        return report(McpState::NotRequired, vec![], None);
    }
    if required.iter().any(|name| name.trim().is_empty()) {
        return report(
            McpState::Unknown,
            vec![],
            Some("The release MCP declaration is incomplete"),
        );
    }
    let mut unique = std::collections::HashSet::new();
    let required: Vec<String> = required
        .iter()
        .filter(|name| unique.insert(name.as_str()))
        .cloned()
        .collect();
    let value = match read_config(path) {
        Ok(Some(value)) => value,
        Ok(None) => {
            return report(
                McpState::Missing,
                required,
                Some("AIPro MCP configuration file was not found"),
            )
        }
        Err(reason) => return report(McpState::Unverifiable, vec![], Some(reason)),
    };
    let Some(servers) = value
        .get("mcpServers")
        .and_then(serde_json::Value::as_object)
    else {
        return report(
            McpState::Unverifiable,
            vec![],
            Some("AIPro MCP configuration must contain a mcpServers object"),
        );
    };
    let missing: Vec<String> = required
        .into_iter()
        .filter(|name| {
            !servers
                .get(name)
                .and_then(serde_json::Value::as_object)
                .is_some_and(|value| !value.is_empty())
        })
        .collect();
    report(
        if missing.is_empty() {
            McpState::Configured
        } else {
            McpState::Missing
        },
        missing,
        None,
    )
}

pub fn inspect_agent_mcp(agent_key: &str, path: &Path, required: Option<&[String]>) -> McpReport {
    if agent_key != "aipro" {
        return report(
            McpState::Unverifiable,
            vec![],
            Some("MCP inspection is available only for AIPro"),
        );
    }
    inspect_mcp(path, required)
}

fn read_config(path: &Path) -> Result<Option<serde_json::Value>, &'static str> {
    use super::targets::{is_reparse, validate_directory_path};
    use std::io::Read;
    const LIMIT: u64 = 1024 * 1024;
    let parent = path
        .parent()
        .ok_or("AIPro MCP configuration location is invalid")?;
    validate_directory_path(parent)
        .map_err(|_| "AIPro MCP configuration directory cannot be verified")?;
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() || is_reparse(&metadata) => {
            return Err("AIPro MCP configuration must be a regular file without links")
        }
        Ok(_) => (),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err("AIPro MCP configuration could not be read"),
    }
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        options.custom_flags(0x00200000);
    } // FILE_FLAG_OPEN_REPARSE_POINT
    let file = options
        .open(path)
        .map_err(|_| "AIPro MCP configuration could not be read")?;
    let metadata = file
        .metadata()
        .map_err(|_| "AIPro MCP configuration could not be verified")?;
    if !metadata.is_file() || is_reparse(&metadata) {
        return Err("AIPro MCP configuration is not a supported regular file");
    }
    if metadata.len() > LIMIT {
        return Err("AIPro MCP configuration exceeds the 1 MiB inspection limit");
    }
    let mut bytes = Vec::new();
    file.take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "AIPro MCP configuration could not be read")?;
    if bytes.len() as u64 > LIMIT {
        return Err("AIPro MCP configuration exceeds the 1 MiB inspection limit");
    }
    serde_json::from_slice(&bytes)
        .map(Some)
        .map_err(|_| "AIPro MCP configuration is not valid JSON")
}

fn report(state: McpState, missing: Vec<String>, reason: Option<&str>) -> McpReport {
    McpReport {
        state,
        missing,
        reason: reason.map(str::to_owned),
        checked_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    }
}
