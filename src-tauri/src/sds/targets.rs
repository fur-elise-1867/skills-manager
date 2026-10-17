use super::{contracts::Target, error::SdsError};
use std::path::Path;

/// Owns destination settings; operation callers resolve again immediately before review/commit.
pub struct TargetService {
    home: std::path::PathBuf,
    settings_root: std::path::PathBuf,
    locked: bool,
    gate: std::sync::Mutex<()>,
}
impl TargetService {
    pub fn new(home: &Path, settings_root: &Path) -> Self {
        Self {
            home: home.into(),
            settings_root: settings_root.into(),
            locked: false,
            gate: std::sync::Mutex::new(()),
        }
    }
    #[cfg(debug_assertions)]
    pub fn fixture(root: &Path) -> Result<Self, SdsError> {
        let home = root.join("agent-home");
        validate_directory_path(&home.join(".aipro"))?;
        std::fs::create_dir_all(home.join(".aipro")).map_err(|_| settings_error())?;
        Ok(Self {
            locked: true,
            ..Self::new(&home, root)
        })
    }
    pub fn settings(&self) -> Result<super::contracts::TargetSettings, SdsError> {
        let _gate = self.gate.lock().map_err(|_| settings_error())?;
        self.view(self.read_override()?)
    }
    pub fn set_override(
        &self,
        root: Option<&Path>,
    ) -> Result<super::contracts::TargetSettings, SdsError> {
        let _gate = self.gate.lock().map_err(|_| settings_error())?;
        if self.locked {
            return Err(SdsError::invalid("Development fixture target is fixed"));
        }
        if let Some(root) = root {
            resolve_aipro_global(&self.home, Some(root))?;
        }
        validate_directory_path(&self.settings_root)?;
        std::fs::create_dir_all(&self.settings_root).map_err(|_| settings_error())?;
        use std::io::Write;
        let mut file = tempfile::Builder::new()
            .prefix(".targets-")
            .tempfile_in(&self.settings_root)
            .map_err(|_| settings_error())?;
        let saved = serde_json::json!({"overrideRoot":root});
        file.write_all(&serde_json::to_vec(&saved).map_err(|_| settings_error())?)
            .and_then(|_| file.as_file().sync_all())
            .map_err(|_| settings_error())?;
        file.persist(self.settings_root.join("targets.json"))
            .map_err(|_| settings_error())?;
        self.view(root.map(Path::to_path_buf))
    }
    /// MCP is always read from the agent's global configuration, never the custom skills folder.
    pub fn inspect_mcp(&self, required: Option<&[String]>) -> super::contracts::McpReport {
        super::mcp::inspect_agent_mcp("aipro", &self.home.join(".aipro/mcp.json"), required)
    }
    fn view(
        &self,
        root: Option<std::path::PathBuf>,
    ) -> Result<super::contracts::TargetSettings, SdsError> {
        let resolved = resolve_aipro_global(&self.home, root.as_deref());
        let (target, error) = match resolved {
            Ok(target) => (Some(target), None),
            Err(error) => (None, Some(error)),
        };
        Ok(super::contracts::TargetSettings {
            override_root: root,
            target,
            error,
            locked: self.locked,
        })
    }
    fn read_override(&self) -> Result<Option<std::path::PathBuf>, SdsError> {
        if self.locked {
            return Ok(None);
        }
        validate_directory_path(&self.settings_root)?;
        let path = self.settings_root.join("targets.json");
        match std::fs::symlink_metadata(&path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Ok(meta) if meta.is_file() && !is_reparse(&meta) && meta.len() <= 65_536 => (),
            _ => return Err(settings_error()),
        }
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        struct Saved {
            override_root: Option<std::path::PathBuf>,
        }
        use std::io::Read;
        let mut options = std::fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            options.custom_flags(0x00200000);
        }
        let file = options.open(path).map_err(|_| settings_error())?;
        if is_reparse(&file.metadata().map_err(|_| settings_error())?) {
            return Err(settings_error());
        }
        let mut bytes = Vec::new();
        file.take(65_537)
            .read_to_end(&mut bytes)
            .map_err(|_| settings_error())?;
        if bytes.len() > 65_536 {
            return Err(settings_error());
        }
        let saved: Saved = serde_json::from_slice(&bytes).map_err(|_| settings_error())?;
        Ok(saved.override_root)
    }
}
fn settings_error() -> SdsError {
    SdsError::new("SETTINGS_SAVE_FAILED", "AIPro destination settings could not be read or saved. Choose a folder again or use the default destination.", false)
}

pub fn resolve_aipro_global(home: &Path, override_root: Option<&Path>) -> Result<Target, SdsError> {
    let candidate = override_root
        .map(Path::to_path_buf)
        .unwrap_or_else(|| home.join(".aipro/skills"));
    validate_directory_path(&candidate)?;
    let path = if let Some(root) = override_root {
        if !root.is_dir() {
            return Err(not_ready());
        }
        root.to_path_buf()
    } else {
        if !home.join(".aipro").is_dir() {
            return Err(not_ready());
        }
        home.join(".aipro/skills")
    };
    Ok(Target {
        agent_key: "aipro".into(),
        workspace_id: "global".into(),
        path,
    })
}

/// Reject aliases and reparse ancestors before following a path or creating files.
/// Missing leaves are allowed; deployment must revalidate under its target lock.
pub(crate) fn validate_directory_path(path: &Path) -> Result<(), SdsError> {
    let text = path.to_str().ok_or_else(unsafe_target)?;
    if !path.is_absolute()
        || text
            .split(['\\', '/'])
            .any(|part| part == "." || part == "..")
    {
        return Err(unsafe_target());
    }
    #[cfg(windows)]
    if !matches!(path.components().next(),Some(std::path::Component::Prefix(prefix)) if matches!(prefix.kind(),std::path::Prefix::Disk(_)))
    {
        return Err(unsafe_target());
    }
    for component in path.components() {
        if let std::path::Component::Normal(name) = component {
            let name = name.to_str().ok_or_else(unsafe_target)?;
            let stem = name.split('.').next().unwrap_or("").to_ascii_uppercase();
            if name.ends_with(['.', ' '])
                || name
                    .chars()
                    .any(|c| c.is_control() || "<>:\"|?*".contains(c))
                || ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
                || (stem.len() == 4
                    && (stem.starts_with("COM") || stem.starts_with("LPT"))
                    && stem.as_bytes()[3].is_ascii_digit()
                    && stem.as_bytes()[3] != b'0')
            {
                return Err(unsafe_target());
            }
        }
    }
    for ancestor in path.ancestors() {
        match std::fs::symlink_metadata(ancestor) {
            Ok(metadata) if is_reparse(&metadata) || !metadata.is_dir() => {
                return Err(unsafe_target())
            }
            Ok(_) => (),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(_) => return Err(unsafe_target()),
        }
    }
    Ok(())
}
pub(crate) fn is_reparse(metadata: &std::fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        false
    }
}
fn unsafe_target() -> SdsError {
    SdsError::new("UNSAFE_TARGET","The destination cannot be validated. Use an absolute local folder without aliases or directory links.",false)
}
fn not_ready() -> SdsError {
    SdsError::new(
        "TARGET_NOT_READY",
        "AIPro is not ready. Open AIPro first or select an existing skills folder in Settings.",
        false,
    )
}
