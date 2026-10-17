# Synthetic archive fixtures

`sds-synthetic.zip` and `sds-synthetic.7z` contain the same single skill root, Unicode text resource and binary resource. They are contract fixtures, never actual published releases. `receipts.json` records the exact archive sizes and SHA-256 values; the existing release API fixture's illustrative digest is unrelated.

Regenerate intentionally from the repository root:

```powershell
cargo run --manifest-path src-tauri/Cargo.toml --no-default-features --features sds --example sds-package-fixtures -- contracts/sds-desktop/v1/fixtures/packages
```

Rust 1.97.1 or newer is required for the SDS 7z reader. The compressor and AES implementation are development dependencies for fixture generation and adversarial tests. Production uses a bounded reader without password support and never invokes a system archiver or runs package scripts.

Product defaults: archive 100 MiB, expanded content 500 MiB, 10,000 entries, 32 path levels. The 7z reader additionally caps decoder memory at 256 MiB and encoded/decoded headers at 8 MiB before allocation. These are proposed pilot limits, not measurements of the real Store. Repack a rejected archive with smaller resources/dictionaries, fewer files or shallower paths; encrypted and split volumes are unsupported.

ZIP uses the file-backed `rawzip` streaming metadata iterator with fixed buffers, an 8 MiB central metadata budget, count checks, CRC and descriptor validation. Supported ZIP methods are Store, Deflate, Deflate64 and Bzip2, with bounded decoder state. Other ZIP methods (including XZ, LZMA and Zstandard) require repacking. A 7z package can use the corresponding codecs supported by its memory-bounded reader. The existing upstream `zip` dependency and its feature set remain unchanged; it is also used to create synthetic test ZIPs.
