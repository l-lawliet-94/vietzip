//! Công cụ tính checksum/hash độc lập trên 1 file bất kỳ (không liên quan tới thao tác
//! archive) — tương đương tính năng CRC/hash của 7-Zip trong menu chuột phải "CRC SHA".
//! Đọc file theo luồng (buffer cố định), không load hết vào RAM — cùng nguyên tắc streaming
//! I/O đã áp dụng cho việc nén/giải nén (NFR-04), quan trọng với file lớn (>1GB, DoD #4).

use crate::{Error, Result};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::Read;
use std::path::Path;

#[derive(Debug, Clone)]
pub struct FileChecksum {
    pub crc32: u32,
    /// Dạng hex thường (64 ký tự), cách hiển thị quen thuộc nhất cho SHA-256.
    pub sha256_hex: String,
    pub size_bytes: u64,
}

pub fn compute_checksum(path: &Path) -> Result<FileChecksum> {
    let mut file = File::open(path).map_err(|e| Error::io(path, e))?;

    let mut crc = crc32fast::Hasher::new();
    let mut sha = Sha256::new();
    let mut size_bytes = 0u64;

    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).map_err(|e| Error::io(path, e))?;
        if n == 0 {
            break;
        }
        crc.update(&buf[..n]);
        sha.update(&buf[..n]);
        size_bytes += n as u64;
    }

    let sha256_hex = sha.finalize().iter().map(|b| format!("{b:02x}")).collect::<String>();

    Ok(FileChecksum {
        crc32: crc.finalize(),
        sha256_hex,
        size_bytes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_content_matches_known_hashes() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("hello.txt");
        std::fs::write(&path, b"hello world").unwrap();

        let result = compute_checksum(&path).unwrap();

        // Giá trị tham chiếu đã xác nhận độc lập: SHA-256 qua .NET
        // System.Security.Cryptography (PowerShell), CRC32 qua Python zlib.crc32 —
        // không tự suy ra từ trí nhớ (lần đầu gõ tay bị thiếu 1 ký tự cuối, phát hiện
        // được nhờ xác minh này).
        assert_eq!(
            result.sha256_hex,
            "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9"
        );
        assert_eq!(result.crc32, 0x0d4a1185);
        assert_eq!(result.size_bytes, 11);
    }

    #[test]
    fn empty_file_has_stable_hashes() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("empty.txt");
        std::fs::write(&path, b"").unwrap();

        let result = compute_checksum(&path).unwrap();
        assert_eq!(result.size_bytes, 0);
        assert_eq!(result.crc32, 0);
        assert_eq!(
            result.sha256_hex,
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[test]
    fn different_content_gives_different_hashes() {
        let tmp = tempfile::tempdir().unwrap();
        let a = tmp.path().join("a.txt");
        let b = tmp.path().join("b.txt");
        std::fs::write(&a, "noi dung a").unwrap();
        std::fs::write(&b, "noi dung b").unwrap();

        let ra = compute_checksum(&a).unwrap();
        let rb = compute_checksum(&b).unwrap();
        assert_ne!(ra.sha256_hex, rb.sha256_hex);
        assert_ne!(ra.crc32, rb.crc32);
    }
}
