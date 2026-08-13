# Nguồn gốc fixture

`ext4view_test_disk_ext3.bin.zst` — ảnh hệ thống tệp ext3 thật, tải nguyên vẹn từ repo GitHub
chính chủ của crate `ext4-view` (`nicholasbishop/ext4-view-rs`, `test_data/test_disk_ext3.bin.zst`),
cùng giấy phép Apache-2.0/MIT với crate — không sửa đổi, không tự dựng bằng tay.

Dùng cho `crates/core/src/ext_format.rs`'s test `list_extract_and_test_roundtrip_on_real_ext3_image`
và `streaming_extract_matches_crate_eager_read` — verify thật trên 1 ảnh ext3 thật (1002 entry:
`lost+found` + `medium_dir` chứa 1000 file nhỏ), thay vì tự dựng 1 ảnh ext2/ext3/ext4 hợp lệ
bằng tay (rủi ro cao hơn nhiều so với header LZH đơn giản ở `lzh_format.rs`, do ext có quá
nhiều cấu trúc liên kết: superblock, block group descriptor, bitmap, inode table, entry thư
mục...). Giải nén NGAY LÚC CHẠY TEST bằng chính crate `zstd` đã có sẵn trong dự án (không commit
bản giải nén) — cùng cách chính `ext4-view` cũng làm với fixture của nó (`src/test_util.rs`).

Nguồn: https://github.com/nicholasbishop/ext4-view-rs/blob/main/test_data/test_disk_ext3.bin.zst

---

`unarc_rs_stored.arj`, `unarc_rs_wrongcrc32.arj`, `unarc_rs_license_crypted.arj` — fixture ARJ
thật, sao chép nguyên vẹn từ nội dung ĐÃ PUBLISH lên crates.io của crate `unarc-rs`
(`tests/arj/stored.arj`/`wrongcrc32.arj`/`license_crypted.arj` — thư mục `tests/` không nằm
trong `exclude` của `Cargo.toml` crate đó nên là nội dung thật đã tải về máy qua `cargo fetch`,
không phải tự dựng bằng tay), cùng giấy phép MIT/Apache-2.0 với crate. Dùng cho
`crates/core/src/arj_format.rs`'s test roundtrip/corruption/password. `stored.arj` chứa đúng 1
entry "LICENSE" (nội dung là giấy phép Apache-2.0 gốc của `unarc-rs`, không nén — method 0);
`wrongcrc32.arj` dùng trong chính bộ test riêng của `unarc-rs` để xác nhận phát hiện lỗi CRC;
`license_crypted.arj` có 1 entry mã hoá, mật khẩu thật không rõ (không có trong bộ test gốc của
crate) — chỉ dùng để xác nhận đường lỗi "chưa có mật khẩu" báo đúng, không thử giải mã.

Nguồn: https://github.com/mkrueger/unarc-rs/tree/main/tests/arj

---

`nsis_deflate_nonsolid.exe`, `nsis_full_featured.exe` — installer NSIS thật, build bằng đúng
trình biên dịch NSIS thật (không phải tự dựng bằng tay), tải nguyên vẹn từ chính repo GitHub
gốc của crate `nsis` (`ATRAPSLLC/nsis-rs`, `tests/fixtures/deflate_nonsolid.exe`/
`full_featured.exe`), cùng giấy phép Apache-2.0 với crate. Kịch bản `.nsi` gốc dùng để build 2
file này cũng có trong repo đó (`tests/build_fixtures/*.nsi`) — dùng để biết chính xác nội dung
nhúng bên trong (2 file "payload.txt"/"config.ini", "payload.txt" nội dung "This is a test
payload for NSIS fixture generation."), không phải đoán. `deflate_nonsolid.exe` nén non-solid
(zlib); `full_featured.exe` nén solid (LZMA) và cố tình nhúng "payload.txt" ở 2 section khác
nhau — dùng để verify `nsis_format.rs::dedupe_name` không ghi đè âm thầm khi 2 entry trùng tên
nguồn. Dùng cho `crates/core/src/nsis_format.rs`'s test.

Nguồn: https://github.com/ATRAPSLLC/nsis-rs/tree/main/tests/fixtures

---

Fixture `.chm` cho `crates/core/src/chm_format.rs` KHÔNG phải file tĩnh commit vào đây — được
tự dựng ngay lúc chạy test (`build_minimal_chm` trong chính file test) vì crate `libchm` không
có API ghi VÀ không có fixture thật nào ở thượng nguồn để mượn (khác mọi định dạng khác ở đây).
Xem doc comment của `build_minimal_chm` trong `chm_format.rs` để biết cách dựng (suy trực tiếp
từ đọc source thật của `libchm::format`, không phải đoán theo tài liệu định dạng CHM bên ngoài).
