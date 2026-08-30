#!/usr/bin/env bash
# AOSP の device-art-resources から Pixel のフレーム素材を取得し、
# back.webp（本体。画面部が透明）の上に mask.webp（カメラ穴と角の覆い）を display オフセット位置に重ねて
# 1 枚の PNG にする。実行時は「スクショの上にフレーム 1 枚」で合成できる形にするのが目的。
# 必要: curl, base64, ImageMagick 7 (magick)
# 使い方: scripts/build-pixel-frames.sh   → src-tauri/frames/google/<id>.png と LICENSE / NOTICE を更新
set -euo pipefail

BASE="https://android.googlesource.com/platform/tools/adt/idea/+/refs/heads/mirror-goog-studio-main/artwork/resources/device-art-resources"
OUT="$(cd "$(dirname "$0")/.." && pwd)/src-tauri/frames/google"
DEVICES=(pixel_9 pixel_9_pro pixel_9_pro_xl pixel_9a pixel_10 pixel_10_pro pixel_10_pro_xl pixel_10a pixel_tablet)

command -v magick >/dev/null || { echo "ImageMagick (magick) が必要です" >&2; exit 1; }
mkdir -p "$OUT"
TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

for d in "${DEVICES[@]}"; do
  for f in back.webp mask.webp layout; do
    curl -fsSL "$BASE/$d/$f?format=TEXT" | base64 -d > "$TMP/$d.$f"
  done
  ox=$(awk '/name device/{f=1} f&&/^ *x /{print $2; exit}' "$TMP/$d.layout")
  oy=$(awk '/name device/{f=1} f&&/^ *y /{print $2; exit}' "$TMP/$d.layout")
  magick "$TMP/$d.back.webp" "$TMP/$d.mask.webp" -geometry "+${ox}+${oy}" -composite "PNG32:$OUT/$d.png"
  echo "$d: $(magick identify -format '%wx%h' "$OUT/$d.png") (mask at +$ox+$oy)"
done

curl -fsSL https://www.apache.org/licenses/LICENSE-2.0.txt -o "$OUT/LICENSE"
cat > "$OUT/NOTICE" <<EOF
Google Pixel device frames in this directory are derived from the Android Open Source Project
(platform/tools/adt/idea, artwork/resources/device-art-resources), licensed under the Apache License 2.0
(see LICENSE in this directory).

Source: $BASE/
Retrieved: $(date +%Y-%m-%d) (branch mirror-goog-studio-main)
Modification: mask.webp composited over back.webp at the display offset given in each device's
"layout" file, then converted to PNG by scripts/build-pixel-frames.sh.
EOF
echo "done: $OUT"
