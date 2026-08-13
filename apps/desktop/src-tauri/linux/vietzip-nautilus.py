"""FR-22 (bản Linux) — menu ngữ cảnh Nautilus, tương đương "Compress to .zip"/"Extract
Here"/"Extract to subfolder" đã làm cho Windows/MSI (xem shell-context-menu.wxs). Gọi ra
chính CLI `vietzip` đã có sẵn (không tự cài lại logic nén/giải nén ở đây).

Cài qua gói .deb (bundle.linux.deb.files trong tauri.conf.json) vào
/usr/share/nautilus-python/extensions/ — cần gói hệ thống `python3-nautilus` đã cài (khai
báo là "recommends", không phải "depends" cứng, vì không phải ai cài Vietzip cũng dùng GNOME
Files/Nautilus).

CHƯA KIỂM THỬ trên máy Nautilus thật (máy dev này không build/chạy được Linux — xem
CLAUDE.md mục Đóng gói Linux). Điểm không chắc chắn lớn nhất: chữ ký phương thức
`get_file_items`/`get_background_items` đổi giữa Nautilus bản GTK3 cũ (nhận thêm tham số
`window` đứng trước) và bản GTK4 mới (Nautilus 43+, vd Ubuntu 24.04 — bỏ tham số `window`).
Dùng `*args` rồi lấy phần tử cuối cùng để tương thích cả 2, đây là kỹ thuật nhiều extension
Nautilus thật ngoài đời cũng dùng cho đúng lý do này, không phải suy đoán tuỳ tiện.
"""

import subprocess

import gi

gi.require_version("Nautilus", "3.0")
from gi.repository import GObject, Nautilus  # noqa: E402

VIETZIP_BIN = "/usr/bin/vietzip"

_ARCHIVE_SUFFIXES = (
    ".zip",
    ".7z",
    ".rar",
    ".tar",
    ".tar.gz",
    ".tgz",
    ".tar.bz2",
    ".tbz2",
    ".tar.zst",
)


def _run_vietzip(args):
    try:
        subprocess.Popen([VIETZIP_BIN] + args)
    except OSError:
        pass


def _is_archive(name):
    return name.lower().endswith(_ARCHIVE_SUFFIXES)


class VietzipExtension(GObject.GObject, Nautilus.MenuProvider):
    def _on_compress(self, _menu, files):
        paths = [f.get_location().get_path() for f in files]
        paths = [p for p in paths if p]
        if paths:
            _run_vietzip(["compress"] + paths)

    def _on_extract_here(self, _menu, path):
        _run_vietzip(["extract", path, "--here"])

    def _on_extract_subfolder(self, _menu, path):
        _run_vietzip(["extract", path, "--to-subfolder"])

    def get_file_items(self, *args):
        files = args[-1]
        if not files:
            return []

        if len(files) == 1 and _is_archive(files[0].get_name()):
            path = files[0].get_location().get_path()
            if not path:
                return []

            extract_here = Nautilus.MenuItem(
                name="Vietzip::ExtractHere",
                label="Extract Here",
                tip="Giai nen ngay tai day bang Vietzip",
            )
            extract_here.connect("activate", self._on_extract_here, path)

            extract_sub = Nautilus.MenuItem(
                name="Vietzip::ExtractSubfolder",
                label="Extract to subfolder",
                tip="Giai nen vao thu muc con cung ten bang Vietzip",
            )
            extract_sub.connect("activate", self._on_extract_subfolder, path)
            return [extract_here, extract_sub]

        compress = Nautilus.MenuItem(
            name="Vietzip::Compress",
            label="Compress to .zip",
            tip="Nen muc da chon thanh .zip bang Vietzip",
        )
        compress.connect("activate", self._on_compress, files)
        return [compress]

    def get_background_items(self, *args):
        current_folder = args[-1]
        path = current_folder.get_location().get_path()
        if not path:
            return []

        item = Nautilus.MenuItem(
            name="Vietzip::CompressFolder",
            label="Compress this folder to .zip",
            tip="Nen thu muc nay thanh .zip bang Vietzip",
        )
        item.connect("activate", lambda _menu: _run_vietzip(["compress", path]))
        return [item]
