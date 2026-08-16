# Third-Party Licenses / Giấy phép bên thứ ba

Vietzip's own source code is licensed under the [MIT License](LICENSE). This
document lists the third-party components it depends on and their license
terms — required reading before redistributing a build, since one component
(UnRAR) carries extra conditions beyond MIT. This file is also embedded
verbatim in the desktop app's About screen.

*(Vietnamese version below / bản tiếng Việt ở dưới.)*

## Rust dependencies

The vast majority of Vietzip's dependencies are ordinary Rust ecosystem
crates under MIT and/or Apache-2.0 — both permissive, both compatible with
MIT redistribution. To generate the full machine-checked list for the
version you're building, run:

```bash
cargo install cargo-license
cargo license --avoid-dev-deps
```

## The one exception: UnRAR (`.rar` extraction)

Vietzip reads `.rar` archives (extract, list, test-integrity only — it never
creates or writes `.rar` files) using RARLAB's own UnRAR source code,
wrapped by the `unrar-ng`/`unrar-ng-sys` and `unrar`/`unrar_sys` crates
(MIT/Apache-2.0 for the Rust wrapper code itself; the underlying UnRAR
source keeps RARLAB's own license). The relevant terms, quoted from
UnRAR's `license.txt`:

> UnRAR source code may be used in any software to handle RAR archives
> without limitations free of charge, but cannot be used to develop RAR
> (WinRAR) compatible archiver and to re-create RAR compression algorithm,
> which is proprietary.

In short: free to use for extraction, never for building a RAR-compatible
compressor — exactly what Vietzip does (extraction only, see FR-02/FR-14 in
the project spec). If you redistribute modified UnRAR source, RARLAB
requires the full license text to travel with it; the software is provided
"AS IS" with no warranty, and RARLAB (Alexander Roshal) retains all
copyright to RAR/UnRAR.

Two small, locally-vendored patches keep this working correctly across
platforms and architectures (calling-convention and cross-compile
target-detection fixes to the Rust FFI bindings — no changes to RARLAB's
own C++ source or its license terms). See `vendor/unrar-ng-sys`,
`vendor/unrar-ng`, `vendor/unrar_sys`, `vendor/unrar` and the root
`Cargo.toml`'s `[patch.crates-io]` section for details.

---

# Bản tiếng Việt

Mã nguồn của Vietzip được cấp phép theo [Giấy phép MIT](LICENSE). Tài liệu
này liệt kê các thành phần bên thứ ba mà dự án phụ thuộc và điều khoản giấy
phép của chúng — cần đọc trước khi phân phối lại bản build, vì có 1 thành
phần (UnRAR) mang điều kiện riêng ngoài MIT. File này cũng được nhúng
nguyên văn vào màn hình About của ứng dụng desktop.

## Dependency Rust

Phần lớn tuyệt đối các dependency của Vietzip là các crate Rust thông
thường theo giấy phép MIT và/hoặc Apache-2.0 — đều là giấy phép permissive,
tương thích với việc phân phối lại theo MIT. Để lấy danh sách đầy đủ, kiểm
tra tự động cho đúng phiên bản đang build, chạy:

```bash
cargo install cargo-license
cargo license --avoid-dev-deps
```

## Ngoại lệ duy nhất: UnRAR (giải nén `.rar`)

Vietzip đọc file `.rar` (chỉ giải nén, liệt kê, kiểm tra toàn vẹn — không
bao giờ tạo hay ghi file `.rar`) bằng chính mã nguồn UnRAR của RARLAB, được
bọc qua crate `unrar-ng`/`unrar-ng-sys` và `unrar`/`unrar_sys` (phần mã Rust
bọc bên ngoài là MIT/Apache-2.0; mã nguồn UnRAR bên trong vẫn giữ nguyên
giấy phép riêng của RARLAB). Điều khoản liên quan, trích từ `license.txt`
của UnRAR:

> UnRAR source code may be used in any software to handle RAR archives
> without limitations free of charge, but cannot be used to develop RAR
> (WinRAR) compatible archiver and to re-create RAR compression algorithm,
> which is proprietary.

Tóm lại: được dùng miễn phí để giải nén, không bao giờ được dùng để xây
dựng 1 công cụ nén tương thích RAR — đúng những gì Vietzip làm (chỉ giải
nén, xem FR-02/FR-14 trong tài liệu đặc tả dự án). Nếu phân phối lại mã
nguồn UnRAR đã sửa đổi, RARLAB yêu cầu phải kèm toàn văn giấy phép; phần
mềm được cung cấp "AS IS", không bảo hành; RARLAB (Alexander Roshal) giữ
toàn bộ bản quyền RAR/UnRAR.

2 bản vá nhỏ, vendor cục bộ trong repo giúp phần này build đúng trên nhiều
nền tảng/kiến trúc (sửa lỗi calling-convention và lỗi nhận diện target khi
cross-compile trong phần Rust FFI bindings — không đụng vào mã nguồn C++
gốc của RARLAB hay điều khoản giấy phép của họ). Xem `vendor/unrar-ng-sys`,
`vendor/unrar-ng`, `vendor/unrar_sys`, `vendor/unrar` và mục
`[patch.crates-io]` trong `Cargo.toml` gốc để biết chi tiết.
