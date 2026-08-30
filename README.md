# ResponsiveShot

<p align="center">
  <!-- License -->
  <a href="LICENSE">
    <img src="https://img.shields.io/github/license/annrie/ResponsiveShot.svg" alt="License">
  </a>
  <!-- Latest release -->
  <a href="https://github.com/annrie/ResponsiveShot/releases/latest">
    <img src="https://img.shields.io/github/v/release/annrie/ResponsiveShot.svg" alt="Latest release">
  </a>
  <!-- Downloads total -->
  <a href="https://github.com/annrie/ResponsiveShot/releases">
    <img src="https://img.shields.io/github/downloads/annrie/ResponsiveShot/total.svg" alt="Total downloads">
  </a>
  <!-- Downloads latest release -->
  <a href="https://github.com/annrie/ResponsiveShot/releases/latest">
    <img src="https://img.shields.io/github/downloads/annrie/ResponsiveShot/latest/total.svg" alt="Latest release downloads">
  </a>
  <!-- Stars -->
  <a href="https://github.com/annrie/ResponsiveShot/stargazers">
    <img src="https://img.shields.io/github/stars/annrie/ResponsiveShot.svg" alt="Stars">
  </a>
</p>

ResponsiveShot is a macOS desktop app for capturing responsive screenshots in bulk.

It is built with Tauri, Vue 3, TypeScript, and Rust. The app launches Chrome, opens a target URL, and saves screenshots for multiple viewport widths.

## Features

- Capture multiple viewport widths in one run
- Capture modes:
  - Full page
  - Viewport
  - Selected element
- PNG screenshot output
- GIF recording output
- Device frames: capture at a real device's resolution and composite into official Apple / Google Pixel bezels, with optional drop shadow (PNG only)
- Optional manual interaction before capture
- Configurable capture delay
- Ratio-based viewport height for viewport captures
  - 16:9
  - 16:10
  - 4:3
  - 3:2
  - 1:1
  - 21:9
  - 9:16
  - Custom ratio
- macOS universal app build support

## Requirements

- macOS
- Node.js
- pnpm
- Rust
- Tauri prerequisites
- Chrome or Chromium available to `headless_chrome`

For universal macOS builds, both Rust targets are required:

```bash
rustup target add aarch64-apple-darwin
rustup target add x86_64-apple-darwin
```

## Setup

```bash
pnpm install
```

## Development

Run the frontend only:

```bash
pnpm run dev
```

Run the Tauri app in development mode:

```bash
pnpm run tauri:dev
```

## Build

Build the frontend:

```bash
pnpm run build
```

Build the Tauri app for the current architecture:

```bash
pnpm run tauri:build
```

Build a universal macOS `.app` bundle:

```bash
pnpm run tauri:build:universal
```

The universal app is generated under:

```text
src-tauri/target/universal-apple-darwin/release/bundle/macos/ResponsiveShot.app
```

## Device frames / デバイスフレーム

**English**

Select devices in the "デバイスフレーム" panel to capture the page at that device's CSS size and pixel ratio and save it composited into the device bezel (`capture_<device>_framed.png`, or `…_framed-shadow.png` with the drop-shadow toggle). Frames are applied to PNG output only.

- **Google Pixel** (Pixel 9 / 9 Pro / 9 Pro XL / 9a / 10 / 10 Pro / 10 Pro XL / 10a / Pixel Tablet) frames are bundled. They are derived from the Android Open Source Project (Apache License 2.0, see `src-tauri/frames/google/NOTICE`). Regenerate with `scripts/build-pixel-frames.sh` (requires ImageMagick).
- **Apple iPhone 16 / 16 Plus / 16 Pro / 16 Pro Max** bezels are not bundled because Apple's license does not allow redistribution. Download the "Product Bezels" DMG from [Apple Design Resources](https://developer.apple.com/design/resources/#product-bezels) yourself and use "DMG / PNG を取り込む" in the panel; the PNGs are copied to `~/Library/Application Support/com.responsiveshot.app/frames/`. Use them under Apple's [marketing guidelines](https://developer.apple.com/app-store/marketing/guidelines/) at your own responsibility (adding a shadow counts as a modification under those guidelines). If the DMG is already mounted in Finder, eject it first, or use 「フォルダを取り込む」 and pick the mounted volume under /Volumes. Use 「取り込み先を Finder で開く」 to reveal that folder.

**日本語**

「デバイスフレーム」パネルで端末を選ぶと、その端末の CSS 寸法・ピクセル比で撮影し、ベゼルにはめ込んだ PNG（`capture_<device>_framed.png`、ドロップシャドウ ON なら `…_framed-shadow.png`）を保存します。フレームは PNG 出力のみに適用されます。

- **Google Pixel**（Pixel 9 / 9 Pro / 9 Pro XL / 9a / 10 / 10 Pro / 10 Pro XL / 10a / Pixel Tablet）のフレームは同梱しています。Android Open Source Project 由来（Apache License 2.0、`src-tauri/frames/google/NOTICE` 参照）。`scripts/build-pixel-frames.sh` で再生成できます（ImageMagick が必要）。
- **Apple iPhone 16 / 16 Plus / 16 Pro / 16 Pro Max** のベゼルは Apple のライセンス上再配布できないため同梱していません。[Apple Design Resources](https://developer.apple.com/design/resources/#product-bezels) から「Product Bezels」の DMG をご自身でダウンロードし、パネルの「DMG / PNG を取り込む」で取り込んでください。PNG は `~/Library/Application Support/com.responsiveshot.app/frames/` にコピーされます。Apple の[マーケティングガイドライン](https://developer.apple.com/app-store/marketing/guidelines/)に従いご自身の責任で使用してください（影の追加はガイドライン上の改変に当たります）。DMG を Finder で既にマウントしている場合は取り出してから取り込むか、「フォルダを取り込む」で /Volumes 内のボリュームを選んでください。「取り込み先を Finder で開く」でそのフォルダを Finder で表示できます。

## Notes

The universal build script currently bundles the macOS `.app` only. This avoids DMG packaging failures while still producing a universal binary.

You can verify the generated binary with:

```bash
lipo -archs src-tauri/target/universal-apple-darwin/release/bundle/macos/ResponsiveShot.app/Contents/MacOS/ResponsiveShot
```

Expected output:

```text
x86_64 arm64
```
