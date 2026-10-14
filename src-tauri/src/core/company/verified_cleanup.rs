use super::*;
use crate::core::company::aipro_paths::validate_ancestors;

// Never recursively remove a tree after checking its hash. Lock each known
// entry first, verify the complete tree while the locks are held, then delete
// only those entries. A new foreign child makes directory removal fail and is
// retained. On Windows, write/rename handles conflict with these locks; deletion
// uses the locked handle rather than a pathname that could be replaced.
pub(super) fn remove_verified_tree(root: &Path, expected: &str) -> CompanyResult<()> {
    validate_ancestors(root)?;
    let mut entries = Vec::new();
    for entry in walkdir::WalkDir::new(root).follow_links(false) {
        let entry = entry.map_err(|_| write_error())?;
        let path = entry.path().to_path_buf();
        validate_ancestors(&path)?;
        let mut options = fs::OpenOptions::new();
        options.read(true);
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            // GENERIC_READ | DELETE, FILE_SHARE_READ (deny write/delete),
            // FILE_FLAG_BACKUP_SEMANTICS | FILE_FLAG_OPEN_REPARSE_POINT.
            options
                .access_mode(0x8000_0000 | 0x0001_0000)
                .share_mode(1)
                .custom_flags(0x0200_0000 | 0x0020_0000);
        }
        let handle = options.open(&path).map_err(|_| write_error())?;
        let metadata = handle.metadata().map_err(|_| write_error())?;
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if metadata.file_attributes() & 0x400 != 0 {
                return Err(write_error());
            }
        }
        if !metadata.is_file() && !metadata.is_dir() {
            return Err(write_error());
        }
        entries.push((path, handle, metadata.is_dir()));
    }
    if content_hash(root, false)? != expected {
        return Err(CompanyError::new(
            "CONTENT_CHANGED",
            "Nội dung cần dọn đã thay đổi; giữ dữ liệu để kiểm tra.",
        ));
    }
    entries.sort_by_key(|(path, _, _)| path.components().count());
    while let Some((path, handle, directory)) = entries.pop() {
        dispose(handle, &path, directory)?;
    }
    Ok(())
}

#[cfg(windows)]
fn dispose(handle: File, _path: &Path, _directory: bool) -> CompanyResult<()> {
    use std::{ffi::c_void, os::windows::io::AsRawHandle};
    #[link(name = "kernel32")]
    extern "system" {
        fn SetFileInformationByHandle(
            file: *mut c_void,
            class: i32,
            info: *mut c_void,
            size: u32,
        ) -> i32;
    }
    let mut delete: u8 = 1; // FILE_DISPOSITION_INFO contains one BOOLEAN.
                            // SAFETY: owned live File handle opened with DELETE; FileDispositionInfo=4,
                            // documented one-byte BOOLEAN layout, writable buffer valid for the call.
                            // https://learn.microsoft.com/windows/win32/api/fileapi/nf-fileapi-setfileinformationbyhandle
    let success = unsafe {
        SetFileInformationByHandle(
            handle.as_raw_handle(),
            4,
            (&mut delete as *mut u8).cast(),
            1,
        )
    };
    if success == 0 {
        return Err(write_error());
    }
    drop(handle); // closes the handle and removes this known directory entry.
    Ok(())
}

#[cfg(not(windows))]
fn dispose(_handle: File, path: &Path, directory: bool) -> CompanyResult<()> {
    // Windows is the supported product platform. Do not recursively unlink
    // newly introduced entries on the development/test fallback either.
    if directory {
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    }
    .map_err(|_| write_error())
}
