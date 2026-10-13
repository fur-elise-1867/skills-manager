use super::types::{CompanyError, CompanyResult, PackageMeta};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{Read, Write},
    path::{Path, PathBuf},
};

pub const MAX_ARCHIVE: u64 = 100 * 1_048_576;
pub const MAX_EXPANDED: u64 = 500 * 1_048_576;
const MAX_ENTRIES: usize = 10_000;
const MAX_DEPTH: usize = 32;

#[derive(Clone)]
pub struct VerifiedPackage {
    pub original_root: PathBuf,
    pub content_hash: String,
    pub full_hash: String,
    pub archive_hash: String,
}
fn invalid(message: &str) -> CompanyError {
    CompanyError::new("INVALID_PACKAGE", message)
}
fn io_error() -> CompanyError {
    CompanyError::new("WRITE_FAILED", "Không đọc/ghi được dữ liệu gói trên máy.")
}

pub fn validate_slug(slug: &str) -> CompanyResult<()> {
    if slug.is_empty()
        || slug.len() > 64
        || !slug.as_bytes()[0].is_ascii_alphanumeric()
        || !slug
            .bytes()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == b'-' || c == b'_')
        || reserved(slug)
    {
        return Err(invalid("Tên thư mục skill không hợp lệ trên Windows."));
    }
    Ok(())
}
fn reserved(name: &str) -> bool {
    let base = name
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (base.len() == 4
            && (base.starts_with("COM") || base.starts_with("LPT"))
            && matches!(base.as_bytes()[3], b'1'..=b'9'))
}
pub fn checked_entry(name: &str) -> CompanyResult<PathBuf> {
    let normalized = name.replace('\\', "/");
    let text = normalized.trim_end_matches('/');
    if text.is_empty() || normalized.starts_with('/') {
        return Err(invalid("Gói có đường dẫn tuyệt đối hoặc rỗng."));
    }
    let parts: Vec<_> = text.split('/').collect();
    if parts.len() > MAX_DEPTH {
        return Err(invalid("Gói vượt giới hạn độ sâu thư mục."));
    }
    for part in &parts {
        if part.is_empty()
            || matches!(*part, "." | "..")
            || part.ends_with('.')
            || part.ends_with(' ')
            || reserved(part)
            || part
                .chars()
                .any(|c| c.is_control() || "<>:\"|?*".contains(c))
        {
            return Err(invalid(
                "Gói có đường dẫn không an toàn hoặc tên file không hợp lệ.",
            ));
        }
    }
    Ok(parts.iter().collect())
}
pub fn reject_links(path: &Path) -> CompanyResult<()> {
    let metadata = fs::symlink_metadata(path).map_err(|_| io_error())?;
    let mut link = metadata.file_type().is_symlink();
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        link |= metadata.file_attributes() & 0x400 != 0;
    }
    if link {
        Err(invalid("Không chấp nhận symlink hoặc junction."))
    } else {
        Ok(())
    }
}
pub fn hash_file(path: &Path) -> CompanyResult<String> {
    let mut file = File::open(path).map_err(|_| io_error())?;
    let mut hash = Sha256::new();
    let mut buffer = [0u8; 65_536];
    loop {
        let count = file.read(&mut buffer).map_err(|_| io_error())?;
        if count == 0 {
            break;
        }
        hash.update(&buffer[..count]);
    }
    Ok(hex::encode(hash.finalize()))
}
pub fn content_hash(root: &Path, comparison: bool) -> CompanyResult<String> {
    let mut records = Vec::new();
    for entry in walkdir::WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| {
            !comparison || !excluded(entry.file_name().to_string_lossy().as_ref())
        })
    {
        let entry = entry.map_err(|_| io_error())?;
        reject_links(entry.path())?;
        if entry.path() == root {
            continue;
        }
        let relative = entry
            .path()
            .strip_prefix(root)
            .map_err(|_| invalid("Đường dẫn nội dung không hợp lệ."))?
            .to_str()
            .ok_or_else(|| invalid("Tên file không đọc được."))?
            .replace('\\', "/");
        if entry.file_type().is_file() {
            records.push((relative, "file", Some(hash_file(entry.path())?)));
        } else if entry.file_type().is_dir() {
            let empty = fs::read_dir(entry.path())
                .map_err(|_| io_error())?
                .next()
                .is_none();
            if !comparison || empty {
                records.push((relative, "directory", None));
            }
        } else {
            return Err(invalid("Nội dung có loại file không hỗ trợ."));
        }
    }
    records.sort_by(|a, b| a.0.cmp(&b.0));
    let encoded =
        serde_json::to_vec(&records).map_err(|_| invalid("Không lập được manifest nội dung."))?;
    Ok(hex::encode(Sha256::digest(encoded)))
}
pub(crate) fn excluded(name: &str) -> bool {
    matches!(name, ".git" | "__pycache__" | "Thumbs.db" | "desktop.ini") || name.ends_with(".pyc")
}

struct EntryPolicy {
    entries: usize,
    expanded: u64,
    names: HashMap<String, String>,
}
impl EntryPolicy {
    fn check(&mut self, name: &str, size: u64) -> CompanyResult<PathBuf> {
        let relative = checked_entry(name)?;
        self.entries += 1;
        self.expanded = self
            .expanded
            .checked_add(size)
            .ok_or_else(|| invalid("Kích thước gói không hợp lệ."))?;
        if self.entries > MAX_ENTRIES || self.expanded > MAX_EXPANDED {
            return Err(invalid(
                "Gói vượt giới hạn số file hoặc dung lượng giải nén.",
            ));
        }
        let spelling = relative.to_string_lossy().replace('\\', "/");
        if self
            .names
            .insert(spelling.to_lowercase(), spelling)
            .is_some()
        {
            return Err(invalid("Gói có entries trùng trên Windows."));
        }
        Ok(relative)
    }
}
fn write_entry(
    input: &mut dyn Read,
    path: &Path,
    directory: bool,
    remaining: &mut u64,
) -> CompanyResult<()> {
    if directory {
        fs::create_dir_all(path).map_err(|_| io_error())?;
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|_| io_error())?;
    }
    let mut output = File::create(path).map_err(|_| io_error())?;
    let mut buffer = [0u8; 65_536];
    loop {
        let count = input
            .read(&mut buffer)
            .map_err(|_| invalid("Không giải nén được nội dung gói."))?;
        if count == 0 {
            break;
        }
        if count as u64 > *remaining {
            return Err(invalid("Dữ liệu thực tế vượt giới hạn giải nén."));
        }
        *remaining -= count as u64;
        output.write_all(&buffer[..count]).map_err(|_| io_error())?;
    }
    output.sync_all().map_err(|_| io_error())
}

pub fn prepare_archive(
    archive: &Path,
    meta: &PackageMeta,
    slug: &str,
    packages: &Path,
) -> CompanyResult<VerifiedPackage> {
    validate_slug(slug)?;
    let length = fs::metadata(archive).map_err(|_| io_error())?.len();
    if length > MAX_ARCHIVE
        || meta.size_bytes != length
        || meta.sha256.len() != 64
        || hash_file(archive)? != meta.sha256
    {
        return Err(CompanyError::new(
            "PACKAGE_INTEGRITY_FAILED",
            "Gói tải không đúng kích thước hoặc SHA-256 đã chọn.",
        ));
    }
    fs::create_dir_all(packages).map_err(|_| io_error())?;
    reject_links(packages)?;
    let scratch = tempfile::tempdir_in(packages).map_err(|_| io_error())?;
    let extract = scratch.path().join("extracted");
    fs::create_dir(&extract).map_err(|_| io_error())?;
    let mut policy = EntryPolicy {
        entries: 0,
        expanded: 0,
        names: HashMap::new(),
    };
    let mut remaining = MAX_EXPANDED;
    match meta.format.as_str() {
        "zip" => {
            let mut zip = zip::ZipArchive::new(File::open(archive).map_err(|_| io_error())?)
                .map_err(|_| invalid("ZIP không hợp lệ."))?;
            if zip.len() > MAX_ENTRIES {
                return Err(invalid("Gói vượt giới hạn số entries."));
            }
            for index in 0..zip.len() {
                let mut entry = zip
                    .by_index(index)
                    .map_err(|_| invalid("Không đọc được ZIP entry."))?;
                let mode = entry.unix_mode().unwrap_or(0) & 0o170000;
                if entry.encrypted() || !matches!(mode, 0 | 0o100000 | 0o040000) {
                    return Err(invalid("ZIP có mã hóa hoặc link/file không hỗ trợ."));
                }
                let relative = policy.check(entry.name(), entry.size())?;
                let directory = entry.is_dir();
                write_entry(
                    &mut entry,
                    &extract.join(relative),
                    directory,
                    &mut remaining,
                )?;
            }
        }
        "7z" => {
            let mut reader =
                sevenz_rust::SevenZReader::open(archive, sevenz_rust::Password::empty())
                    .map_err(|_| invalid("7z không hợp lệ hoặc đã mã hóa."))?;
            for folder in &reader.archive().folders {
                for coder in &folder.coders {
                    let properties = &coder.properties;
                    let dictionary = match coder.decompression_method_id() {
                        [0x21] if properties.len() == 1 && properties[0] < 40 => {
                            (2u64 | (properties[0] as u64 & 1)) << (properties[0] as u64 / 2 + 11)
                        }
                        [0x03, 0x01, 0x01] if properties.len() >= 5 => {
                            u32::from_le_bytes(properties[1..5].try_into().unwrap()) as u64
                        }
                        [0x21] => u64::MAX,
                        _ => 0,
                    };
                    if dictionary > MAX_EXPANDED {
                        return Err(invalid("7z yêu cầu quá nhiều bộ nhớ giải nén."));
                    }
                }
            }
            for entry in &reader.archive().files {
                if entry.is_anti_item
                    || entry.windows_attributes & 0x400 != 0
                    || (entry.windows_attributes >> 16) & 0o170000 == 0o120000
                {
                    return Err(invalid("7z có link hoặc anti-item không hỗ trợ."));
                }
                policy.check(&entry.name, entry.size)?;
            }
            reader
                .for_each_entries(|entry, input| {
                    let path = checked_entry(&entry.name)
                        .map_err(|_| sevenz_rust::Error::other("unsafe path"))?;
                    write_entry(
                        input,
                        &extract.join(path),
                        entry.is_directory,
                        &mut remaining,
                    )
                    .map_err(|_| sevenz_rust::Error::other("invalid or oversized entry"))?;
                    Ok(true)
                })
                .map_err(|_| invalid("Không giải nén được 7z hoặc gói vượt giới hạn."))?;
        }
        _ => return Err(invalid("Định dạng gói phải là ZIP hoặc 7z.")),
    }
    let skill_root = if extract.join("SKILL.md").is_file() {
        extract.clone()
    } else {
        let entries: Vec<_> = fs::read_dir(&extract)
            .map_err(|_| io_error())?
            .collect::<Result<_, _>>()
            .map_err(|_| io_error())?;
        let wrapped = extract.join(slug);
        if entries.len() != 1 || !wrapped.join("SKILL.md").is_file() {
            return Err(invalid(
                "Gói phải có một SKILL.md ở root hoặc wrapper khớp slug.",
            ));
        }
        wrapped
    };
    let marker = fs::read_to_string(skill_root.join("SKILL.md"))
        .map_err(|_| invalid("SKILL.md phải là UTF-8 đọc được."))?;
    if marker.trim().is_empty() {
        return Err(invalid("SKILL.md không được rỗng."));
    }
    let full = content_hash(&skill_root, false)?;
    let comparison = content_hash(&skill_root, true)?;
    let destination = packages.join(&meta.sha256);
    if destination.exists() {
        reject_links(&destination)?;
        if content_hash(&destination.join("original"), false)? != full {
            return Err(invalid(
                "Bản gốc trong cache đã thay đổi. Kiểm tra dữ liệu app trước khi thử lại.",
            ));
        }
    } else {
        let prepared = scratch.path().join("package");
        fs::create_dir(&prepared).map_err(|_| io_error())?;
        fs::rename(&skill_root, prepared.join("original")).map_err(|_| io_error())?;
        fs::copy(archive, prepared.join(format!("archive.{}", meta.format)))
            .map_err(|_| io_error())?;
        fs::write(
            prepared.join("manifest.json"),
            serde_json::json!({"contentHash":comparison,"fullHash":full,"algorithm":"content-v1"})
                .to_string(),
        )
        .map_err(|_| io_error())?;
        fs::rename(prepared, &destination).map_err(|_| io_error())?;
    }
    Ok(VerifiedPackage {
        original_root: destination.join("original"),
        content_hash: comparison,
        full_hash: full,
        archive_hash: meta.sha256.clone(),
    })
}

pub fn copy_tree(source: &Path, target: &Path) -> CompanyResult<()> {
    fs::create_dir_all(target).map_err(|_| io_error())?;
    for entry in walkdir::WalkDir::new(source).follow_links(false) {
        let entry = entry.map_err(|_| io_error())?;
        reject_links(entry.path())?;
        if entry.path() == source {
            continue;
        }
        let relative = entry.path().strip_prefix(source).map_err(|_| io_error())?;
        let path = target.join(relative);
        if entry.file_type().is_dir() {
            fs::create_dir_all(path).map_err(|_| io_error())?;
        } else if entry.file_type().is_file() {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|_| io_error())?;
            }
            fs::copy(entry.path(), &path).map_err(|_| io_error())?;
            File::options()
                .write(true)
                .open(path)
                .map_err(|_| io_error())?
                .sync_all()
                .map_err(|_| io_error())?;
        } else {
            return Err(invalid("Không sao chép loại file không hỗ trợ."));
        }
    }
    Ok(())
}
