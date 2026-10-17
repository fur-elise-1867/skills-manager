use super::{invalid, limit_error, Extraction};
use crate::sds::error::SdsError;
use sevenz_turbo::{ArchiveLimits, ArchiveReader, Error, Password};
use std::fs::File;

pub(super) fn extract(file: File, sink: &mut Extraction<'_>) -> Result<(), SdsError> {
    let limits = ArchiveLimits {
        memory_limit_bytes: sink.limits.decoder_memory_bytes,
        max_end_header_bytes: sink.limits.header_bytes,
        max_header_unpacked_bytes: sink.limits.header_bytes,
        max_entries: sink.limits.entries as u64,
        max_name_bytes: 8192,
        max_total_name_bytes: sink.limits.header_bytes,
        max_total_coders: (sink.limits.entries as u64).saturating_mul(8),
        max_unpack_bytes: sink.limits.expanded_bytes,
        reject_unsafe_paths: true,
        ..ArchiveLimits::default()
    };
    let mut archive =
        ArchiveReader::with_limits(file, Password::empty(), limits).map_err(map_error)?;
    for entry in &archive.archive().files {
        let attributes = entry.windows_attributes;
        let unix_kind = (attributes >> 16) & 0o170000;
        if entry.is_anti_item
            || (entry.has_windows_attributes
                && (attributes & 0x400 != 0 || ![0, 0o040000, 0o100000].contains(&unix_kind)))
        {
            return Err(invalid());
        }
    }
    if archive
        .archive()
        .blocks
        .iter()
        .flat_map(|block| &block.coders)
        .any(|coder| coder.encoder_method_id() == [0x06, 0xf1, 0x07, 0x01])
    {
        return Err(invalid());
    }
    let mut callback_error = None;
    let result = archive.for_each_entries(|entry, reader| {
        match sink.add(&entry.name, entry.is_directory, entry.size, reader) {
            Ok(()) => Ok(true),
            Err(error) => {
                callback_error = Some(error);
                Err(Error::Other("Package entry refused".into()))
            }
        }
    });
    if let Some(error) = callback_error {
        return Err(error);
    }
    result.map_err(map_error)
}
fn map_error(error: Error) -> SdsError {
    match error {
        Error::LimitExceeded { .. }
        | Error::MemoryLimited { .. }
        | Error::MaxMemLimited { .. }
        | Error::EndHeaderTooLarge { .. } => limit_error(),
        _ => invalid(),
    }
}
