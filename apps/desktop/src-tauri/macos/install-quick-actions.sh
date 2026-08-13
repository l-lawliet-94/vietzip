#!/bin/sh
# FR-22 (macOS) — cai 3 Quick Action (.workflow) vao ~/Library/Services/ de Finder hien
# chung trong menu chuot phai (Quick Actions / Services). Chay 1 lan sau khi da co
# `vietzip` (CLI) tren PATH (vd /usr/local/bin/vietzip hoac /opt/homebrew/bin/vietzip).
#
# CHUA CHAY THU TREN MAC THAT — may dev hien tai la Windows, khong build/test duoc ban
# macOS (xem CLAUDE.md). Neu cac Quick Action khong hien ra sau khi chay script nay, xem
# README.md cung thu muc de tao lai bang tay qua Automator (chac chan dung 100%).
set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
SRC_DIR="$SCRIPT_DIR/quick-actions"
DEST_DIR="$HOME/Library/Services"

if [ ! -d "$SRC_DIR" ]; then
  echo "Khong tim thay $SRC_DIR" >&2
  exit 1
fi

mkdir -p "$DEST_DIR"

for workflow in "$SRC_DIR"/*.workflow; do
  name="$(basename "$workflow")"
  echo "Cai: $name"
  rm -rf "$DEST_DIR/$name"
  cp -R "$workflow" "$DEST_DIR/$name"
done

if ! command -v vietzip >/dev/null 2>&1; then
  echo ""
  echo "CANH BAO: khong tim thay lenh 'vietzip' trong PATH."
  echo "Cac Quick Action vua cai se hien ra trong Finder nhung se bao loi khi bam vao"
  echo "cho toi khi ban dat CLI 'vietzip' vao PATH (vd /usr/local/bin/vietzip)."
fi

echo ""
echo "Da cai xong. Neu Finder chua thay Quick Action moi ngay, thu dang xuat/dang nhap lai."
