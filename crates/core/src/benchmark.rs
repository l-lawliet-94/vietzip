//! Benchmark tốc độ nén/giải nén — tương đương "Tools > Benchmark" của 7-Zip, ở dạng đơn
//! giản: nén/giải nén 1 khối dữ liệu tổng hợp kích thước cố định qua ZIP và 7Z, đo MB/s và
//! tỉ lệ nén. Không đo điểm CPU tổng hợp (rating) như 7-Zip thật — chỉ đo tốc độ thực tế
//! trên chính core engine của dự án, đủ để người dùng so sánh máy/mức nén, không cần mô
//! phỏng lại thuật toán tính điểm riêng của 7-Zip.

use crate::{CompressOptions, CompressionLevel, Error, Result};
use std::path::Path;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct BenchmarkResult {
    pub format: &'static str,
    pub input_bytes: u64,
    pub compressed_bytes: u64,
    pub compress_mb_per_sec: f64,
    pub decompress_mb_per_sec: f64,
}

impl BenchmarkResult {
    /// Tỉ lệ nén dạng phần trăm kích thước còn lại so với gốc (thấp hơn = nén tốt hơn).
    pub fn compression_ratio_percent(&self) -> f64 {
        (self.compressed_bytes as f64 / self.input_bytes as f64) * 100.0
    }
}

/// Dữ liệu tổng hợp có thể nén được nhưng không lặp lại đơn điệu (cùng cách tạo dữ liệu
/// test đã dùng ở `compression_level_tests`) — dữ liệu toàn số 0 hoặc lặp lại quá dễ sẽ cho
/// tốc độ/tỉ lệ nén không thực tế, không phản ánh đúng hiệu năng trên dữ liệu thật.
fn synthetic_data(size_bytes: usize) -> Vec<u8> {
    let mut data = Vec::with_capacity(size_bytes);
    let words = ["xin", "chao", "viet", "nam", "nen", "giai", "file", "du", "lieu", "toc", "do"];
    let mut seed: u32 = 987654321;
    while data.len() < size_bytes {
        seed = seed.wrapping_mul(1103515245).wrapping_add(12345);
        let word = words[(seed as usize / 65536) % words.len()];
        data.extend_from_slice(word.as_bytes());
        data.push(b' ');
    }
    data.truncate(size_bytes);
    data
}

/// Chạy benchmark trên `size_mb` MB dữ liệu tổng hợp, qua cả ZIP và 7Z, mức nén `level`.
pub fn run_benchmark(size_mb: u64, level: CompressionLevel) -> Result<Vec<BenchmarkResult>> {
    let size_bytes = (size_mb.max(1) * 1024 * 1024) as usize;
    let data = synthetic_data(size_bytes);

    let tmp = tempfile::tempdir().map_err(|e| Error::io(Path::new("benchmark"), e))?;
    let src = tmp.path().join("bench.dat");
    std::fs::write(&src, &data).map_err(|e| Error::io(&src, e))?;

    let mut results = Vec::with_capacity(2);
    for (ext, format) in [("zip", "ZIP"), ("7z", "7Z")] {
        let archive = tmp.path().join(format!("bench.{ext}"));
        let options = CompressOptions { password: None, level };

        let t0 = Instant::now();
        crate::compress(&[src.clone()], &archive, &options)?;
        let compress_secs = t0.elapsed().as_secs_f64().max(f64::MIN_POSITIVE);

        let compressed_bytes = std::fs::metadata(&archive)
            .map_err(|e| Error::io(&archive, e))?
            .len();

        let dest_dir = tmp.path().join(format!("out_{ext}"));
        let t1 = Instant::now();
        crate::extract(&archive, &dest_dir, None)?;
        let decompress_secs = t1.elapsed().as_secs_f64().max(f64::MIN_POSITIVE);

        let mb = size_bytes as f64 / 1_000_000.0;
        results.push(BenchmarkResult {
            format,
            input_bytes: size_bytes as u64,
            compressed_bytes,
            compress_mb_per_sec: mb / compress_secs,
            decompress_mb_per_sec: mb / decompress_secs,
        });
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_runs_and_reports_plausible_numbers() {
        let results = run_benchmark(4, CompressionLevel::Normal).unwrap();
        assert_eq!(results.len(), 2);
        for r in &results {
            assert!(r.compress_mb_per_sec > 0.0, "{}: tốc độ nén phải > 0", r.format);
            assert!(r.decompress_mb_per_sec > 0.0, "{}: tốc độ giải nén phải > 0", r.format);
            assert!(r.compressed_bytes > 0, "{}: kích thước nén phải > 0", r.format);
            assert!(
                r.compression_ratio_percent() < 100.0,
                "{}: dữ liệu tổng hợp phải nén được (< 100%), thực tế {:.1}%",
                r.format,
                r.compression_ratio_percent()
            );
        }
    }
}
