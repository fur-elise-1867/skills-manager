//! Synthetic contract artifacts only. Never used by the desktop runtime or installer.
use sha2::{Digest, Sha256};
use std::{error::Error, io::{Cursor, Write}, path::PathBuf};

fn main() -> Result<(), Box<dyn Error>> {
    let out = PathBuf::from(std::env::args_os().nth(1).ok_or("Supply the contract fixture output folder")?);
    std::fs::create_dir_all(&out)?;
    let entries: &[(&str, &[u8])] = &[
        ("SKILL.md", b"---\nname: sds-synthetic-fixture\ndescription: Synthetic package for desktop contract tests.\n---\n# Fixture\nNever a real Store release.\n"),
        ("resources/hướng-dẫn.txt", "Tài nguyên thử nghiệm.\n".as_bytes()),
        ("resources/data.bin", &[0, 255, 17, 0]),
    ];
    let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
    let mut seven = sevenz_turbo::ArchiveWriter::new(Cursor::new(Vec::new()))?;
    for (name, content) in entries {
        zip.start_file(*name, zip::write::SimpleFileOptions::default())?; zip.write_all(content)?;
        seven.push_archive_entry(sevenz_turbo::ArchiveEntry::new_file(name), Some(Cursor::new(*content)))?;
    }
    let mut receipts = Vec::new();
    for (format, bytes) in [("zip", zip.finish()?.into_inner()), ("7z", seven.finish()?.into_inner())] {
        let name = format!("sds-synthetic.{format}");
        std::fs::write(out.join(&name), &bytes)?;
        receipts.push(serde_json::json!({"format":format,"fileName":name,"sizeBytes":bytes.len(),"sha256":hex::encode(Sha256::digest(&bytes))}));
    }
    std::fs::write(out.join("receipts.json"), serde_json::to_vec_pretty(&receipts)?)?;
    Ok(())
}
