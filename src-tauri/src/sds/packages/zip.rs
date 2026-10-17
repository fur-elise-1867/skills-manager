use super::{invalid, limit_error, Extraction};
use crate::sds::error::SdsError;
use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
};

pub(super) fn extract(mut file: File, sink: &mut Extraction<'_>) -> Result<(), SdsError> {
    // Read the fixed-size footer first, before the ZIP reader reserves entry metadata.
    let len = file.metadata().map_err(|_| invalid())?.len();
    let window = len.min(65_557) as usize;
    file.seek(SeekFrom::End(-(window as i64)))
        .map_err(|_| invalid())?;
    let mut tail = vec![0u8; window];
    file.read_exact(&mut tail).map_err(|_| invalid())?;
    let at = (0..tail.len().saturating_sub(21))
        .rev()
        .find(|&i| {
            tail[i..].starts_with(b"PK\x05\x06")
                && i + 22 + u16::from_le_bytes([tail[i + 20], tail[i + 21]]) as usize == tail.len()
        })
        .ok_or_else(invalid)?;
    let footer = &tail[at..];
    let u16_at = |i| u16::from_le_bytes([footer[i], footer[i + 1]]);
    let mut count = u16_at(10) as u64;
    if u16_at(4) != 0 || u16_at(6) != 0 || u16_at(8) as u64 != count {
        return Err(invalid());
    }
    let u32_at = |i| u32::from_le_bytes(footer[i..i + 4].try_into().unwrap()) as u64;
    let mut central_size = u32_at(12);
    let mut central_offset = u32_at(16);
    let footer_offset = len - window as u64 + at as u64;
    let mut directory_end = footer_offset;
    let needs_zip64 =
        count == 65_535 || central_size == u32::MAX as u64 || central_offset == u32::MAX as u64;
    let mut locator = [0u8; 20];
    let has_zip64 = if let Some(offset) = footer_offset.checked_sub(20) {
        file.seek(SeekFrom::Start(offset)).map_err(|_| invalid())?;
        file.read_exact(&mut locator).map_err(|_| invalid())?;
        &locator[..4] == b"PK\x06\x07"
    } else {
        false
    };
    if needs_zip64 && !has_zip64 {
        return Err(invalid());
    }
    if has_zip64 {
        let locator_offset = footer_offset.checked_sub(20).ok_or_else(invalid)?;
        if &locator[..4] != b"PK\x06\x07"
            || locator[4..8] != [0; 4]
            || locator[16..20] != 1u32.to_le_bytes()
        {
            return Err(invalid());
        }
        let offset = u64::from_le_bytes(locator[8..16].try_into().unwrap());
        if offset
            .checked_add(56)
            .is_none_or(|end| end > locator_offset)
        {
            return Err(invalid());
        }
        file.seek(SeekFrom::Start(offset)).map_err(|_| invalid())?;
        let mut record = [0u8; 56];
        file.read_exact(&mut record).map_err(|_| invalid())?;
        let u64_at = |i| u64::from_le_bytes(record[i..i + 8].try_into().unwrap());
        let record_size = u64_at(4);
        if &record[..4] != b"PK\x06\x06"
            || record[16..24] != [0; 8]
            || record_size < 44
            || offset
                .checked_add(record_size)
                .and_then(|n| n.checked_add(12))
                .is_none_or(|end| end != locator_offset)
        {
            return Err(invalid());
        }
        if record_size > sink.limits.header_bytes {
            return Err(limit_error());
        }
        if (count != 65_535 && count != u64_at(32))
            || (central_size != u32::MAX as u64 && central_size != u64_at(40))
            || (central_offset != u32::MAX as u64 && central_offset != u64_at(48))
        {
            return Err(invalid());
        }
        count = u64_at(32);
        if u64_at(24) != count {
            return Err(invalid());
        }
        central_size = u64_at(40);
        central_offset = u64_at(48);
        directory_end = offset;
    }
    if count > sink.limits.entries as u64 {
        return Err(limit_error());
    }
    if central_offset
        .checked_add(central_size)
        .is_none_or(|end| end > directory_end)
        || count.saturating_mul(46) > central_size
    {
        return Err(invalid());
    }
    if central_size > sink.limits.header_bytes {
        return Err(limit_error());
    }
    // Streaming metadata has a fixed buffer and never reserves attacker-declared entry counts.
    let mut policy_file = file.try_clone().map_err(|_| invalid())?;
    let mut buffer = vec![0u8; rawzip::MAX_CENTRAL_DIRECTORY_RECORD_SIZE];
    let mut local_buffer = vec![0u8; rawzip::MAX_CENTRAL_DIRECTORY_RECORD_SIZE];
    let archive = rawzip::ZipArchive::from_file(file, &mut buffer).map_err(|_| invalid())?;
    if archive.entries_hint() != count
        || archive.directory_offset() != central_offset
        || archive.eocd_offset() != footer_offset
        || archive.end_offset() != len
    {
        return Err(invalid());
    }
    let mut entries = archive.entries(&mut buffer);
    let mut seen = 0u64;
    let mut cursor = central_offset;
    let mut ranges = std::collections::BTreeMap::new();
    while let Some(record) = entries.next_entry().map_err(|_| invalid())? {
        seen += 1;
        if seen > sink.limits.entries as u64 {
            return Err(limit_error());
        }
        let raw_path = record.file_path();
        let raw_name = raw_path.as_ref();
        if record.central_directory_offset() != cursor {
            return Err(invalid());
        }
        let metadata_bytes = 46
            + raw_name.len() as u64
            + record.extra_fields().remaining_bytes().len() as u64
            + record.comment().as_bytes().len() as u64;
        cursor = cursor.checked_add(metadata_bytes).ok_or_else(limit_error)?;
        if cursor - central_offset > sink.limits.header_bytes {
            return Err(limit_error());
        }
        if cursor > central_offset + central_size {
            return Err(invalid());
        }
        // rawzip parses these unsupported multipart fields but does not reject them.
        policy_file
            .seek(SeekFrom::Start(record.central_directory_offset() + 34))
            .map_err(|_| invalid())?;
        let mut disk = [0; 2];
        policy_file.read_exact(&mut disk).map_err(|_| invalid())?;
        if disk != [0, 0] {
            return Err(invalid());
        }
        let mode = record.mode();
        if record.flags().is_encrypted()
            || ![0, 0o040000, 0o100000].contains(&(mode.value() & 0o170000))
            || record.external_attributes() & 0x400 != 0
        {
            return Err(invalid());
        }
        let name = std::str::from_utf8(raw_name).map_err(|_| invalid())?;
        let entry = archive
            .get_entry(record.wayfinder())
            .map_err(|_| invalid())?;
        let local = entry
            .local_header(&mut local_buffer)
            .map_err(|_| invalid())?;
        if local.file_path().as_ref() != raw_name
            || local.flags().is_encrypted()
            || local.compression_method() != record.compression_method()
        {
            return Err(invalid());
        }
        let (start, end) = entry.compressed_data_range();
        if start > end || end > central_offset {
            return Err(invalid());
        }
        if start != end {
            if ranges
                .range(..=start)
                .next_back()
                .is_some_and(|(_, previous_end)| *previous_end > start)
                || ranges
                    .range(start..)
                    .next()
                    .is_some_and(|(next_start, _)| *next_start < end)
            {
                return Err(invalid());
            }
            ranges.insert(start, end);
        }
        let raw = entry.reader();
        if let Some(descriptor) = raw.data_descriptor().map_err(|_| invalid())? {
            if descriptor.crc32() != record.crc32()
                || descriptor.compressed_size() != record.compressed_size_hint()
                || descriptor.uncompressed_size() != record.uncompressed_size_hint()
            {
                return Err(invalid());
            }
        }
        let mut decoded: Box<dyn Read + '_> = match record.compression_method() {
            rawzip::CompressionMethod::STORE => Box::new(raw),
            rawzip::CompressionMethod::DEFLATE => Box::new(flate2::read::DeflateDecoder::new(raw)),
            rawzip::CompressionMethod::DEFLATE64 => Box::new(deflate64::Deflate64Decoder::new(raw)),
            rawzip::CompressionMethod::BZIP2 => Box::new(bzip2::read::BzDecoder::new(raw)),
            _ => return Err(SdsError::new("PACKAGE_INVALID", "This ZIP codec cannot be decoded within the pilot memory budget. Repack using Store, Deflate, Deflate64 or Bzip2, or use a bounded 7z package.", false)),
        };
        let mut checked = CrcReader {
            inner: &mut decoded,
            crc: rawzip::Crc32::new(),
        };
        sink.add(
            name,
            record.is_dir(),
            record.uncompressed_size_hint(),
            &mut checked,
        )?;
        if checked.crc.checksum() != record.crc32() {
            return Err(invalid());
        }
    }
    if seen != count || cursor != central_offset + central_size {
        return Err(invalid());
    }
    Ok(())
}

struct CrcReader<R> {
    inner: R,
    crc: rawzip::Crc32,
}
impl<R: Read> Read for CrcReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
        let size = self.inner.read(buffer)?;
        self.crc.update(&buffer[..size]);
        Ok(size)
    }
}
