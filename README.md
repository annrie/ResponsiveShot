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
