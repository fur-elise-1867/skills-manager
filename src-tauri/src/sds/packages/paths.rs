use super::{invalid, limit_error};
use crate::sds::error::SdsError;
use std::collections::BTreeMap;
use unicode_normalization::UnicodeNormalization;

pub(crate) fn relative_path(name: &str, directory: bool, depth: usize) -> Result<String, SdsError> {
    let name = if directory {
        name.strip_suffix('/').unwrap_or(name)
    } else {
        name
    };
    if name.is_empty() || name.contains('\\') || name.len() > 4096 {
        return Err(invalid());
    }
    let parts: Vec<_> = name.split('/').collect();
    if parts.len() > depth {
        return Err(limit_error());
    }
    for part in parts {
        let stem = part.split('.').next().unwrap_or("").to_uppercase();
        if part.is_empty()
            || part == "."
            || part == ".."
            || part.ends_with(['.', ' '])
            || part.encode_utf16().count() > 255
            || part
                .chars()
                .any(|c| c.is_control() || "<>:\"|?*".contains(c))
            || ["CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$"].contains(&stem.as_str())
            || ((stem.starts_with("COM") || stem.starts_with("LPT"))
                && stem.chars().count() == 4
                && matches!(stem.chars().last(), Some('1'..='9' | '¹' | '²' | '³')))
        {
            return Err(invalid());
        }
    }
    Ok(name.into())
}

#[derive(Default)]
pub(super) struct Names {
    values: BTreeMap<String, (String, bool, bool)>,
}
impl Names {
    pub(super) fn insert(&mut self, name: &str, directory: bool) -> Result<(), SdsError> {
        let parts: Vec<_> = name.split('/').collect();
        for i in 1..=parts.len() {
            let path = parts[..i].join("/");
            let key = caseless::default_case_fold_str(&path.nfc().collect::<String>());
            let explicit = i == parts.len();
            let is_dir = !explicit || directory;
            match self.values.get_mut(&key) {
                Some((original, was_dir, was_explicit)) => {
                    if *original != path || !*was_dir || !is_dir || (*was_explicit && explicit) {
                        return Err(invalid());
                    }
                    *was_explicit |= explicit;
                }
                None => {
                    self.values.insert(key, (path, is_dir, explicit));
                }
            }
        }
        Ok(())
    }
    pub(super) fn paths(&self) -> impl Iterator<Item = &str> {
        self.values.values().map(|v| v.0.as_str())
    }
    pub(super) fn directories(&self) -> impl Iterator<Item = &str> {
        self.values.values().filter(|v| v.1).map(|v| v.0.as_str())
    }
}
