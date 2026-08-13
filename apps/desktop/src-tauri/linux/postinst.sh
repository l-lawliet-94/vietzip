#!/bin/sh
# FR-22 (bản Linux) — chạy sau khi cài gói .deb. Nautilus chỉ nạp lại extension Python
# (vietzip-nautilus.py, xem file cùng thư mục) sau khi khởi động lại tiến trình của nó;
# "nautilus -q" là cách chuẩn để yêu cầu việc đó, Nautilus tự khởi động lại khi cần.
# Bỏ qua im lặng nếu máy không cài Nautilus (không phải môi trường GNOME) — không được để
# việc thiếu Nautilus làm cài đặt gói .deb thất bại.
set -e

if command -v nautilus >/dev/null 2>&1; then
  nautilus -q >/dev/null 2>&1 || true
fi

exit 0
