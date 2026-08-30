# デバイスフレーム合成機能 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** 撮影したスクリーンショットを Apple / Google 公式のデバイスフレームにはめ込んだ PNG（ドロップシャドウ有無を選択可）として出力できるようにする。

**Architecture:** フレームのメタデータは `src-tauri/frames/catalog.json`、画像は Google 分を同梱・Apple 分をユーザーが公式 DMG から取り込む。Rust 側に `frames/` モジュール（catalog / store / import / compose）を新設し、既存のキャプチャループを「幅ターゲット + デバイスターゲット」の列挙に一般化して、デバイスは CSS 寸法・DPR・mobile を指定して viewport 撮影 → 合成 → 保存する。フロントは `DeviceFramePanel.vue` を追加して `App.vue` から `v-model` で繋ぐ。

**Tech Stack:** Tauri v2 / Rust（`image` 0.24.4, `serde`, `tauri-plugin-opener` 2）/ Vue 3.5 + TypeScript + UnoCSS / VueUse `useStorage` / ImageMagick（開発時のフレーム生成スクリプトのみ）

**Spec:** `docs/superpowers/specs/2026-08-30-device-frame-design.md`

## Global Constraints

- 素材は公式配布のみ。Apple のベゼル PNG は**リポジトリにもアプリにも含めない**（ライセンス上再配布不可）。Google Pixel は Apache 2.0 なので `src-tauri/frames/google/` に生成物をコミットし `LICENSE` / `NOTICE` を同梱する
- 既存の幅指定キャプチャの出力（ファイル名 `capture_{幅}px_{mode}.png` / `capture_{幅}x{高さ}_{mode}.png`、画素）は **1 px も変えない**。幅ターゲットは `dpr 1.0` / `mobile false`
- デバイスターゲットは常に `viewport` 撮影・PNG のみ。GIF (`duration > 0`) と同時指定された場合 Rust は `Err("デバイスフレームは PNG 出力のみ対応しています")`
- 出力名: `capture_<deviceId>_framed.png`、Apple は `capture_<deviceId>_<variant-slug>_framed.png`、影付きは末尾が `_framed-shadow.png`
- 色名は slug（小文字・空白→`-`、例 `black-titanium`）で統一し、元表記は保持しない
- シャドウ: `sigma = 0.015 × frame.width`（最小 1.0）、`offset_y = round(0.015 × frame.height)`、`opacity = 0.35`、`padding = ceil(3·sigma + offset_y)`。ぼかしは 1/4 縮小で行う
- 新規依存は `tauri-plugin-opener`（Rust + npm）のみ。`regex` / `tempfile` 等は追加しない。`let ... else` は使わない（`Cargo.toml` の `rust-version = "1.60"` を尊重）
- `cargo test` は `tauri::generate_context!()` の都合で `dist/` が必要。無ければ先に `pnpm run build`
- AGENTS.md の既存制約（別スレッドでの `drop(browser)`、`ABORT_FLAG`、手動操作 UI）は触らない
- コミットメッセージは既存の流儀（`<type>: <emoji> 日本語`）。README への追記は日英併記

## File Structure

| パス | 責務 |
|---|---|
| `src-tauri/src/frames/mod.rs` | サブモジュール宣言と共通型 `Rect` |
| `src-tauri/src/frames/compose.rs` | 純関数の合成器: `cover_resize` / `compose_frame` / `ShadowParams` / `shadow_layer` |
| `src-tauri/src/frames/catalog.rs` | `catalog.json` の型（`DeviceEntry` 等）、`parse_catalog` / `load_catalog` / `validate` / `find` |
| `src-tauri/src/frames/store.rs` | 保存場所 `Roots`、`slugify`、`FrameStatus` / `status_for`、`resolve_frame_png` |
| `src-tauri/src/frames/import.rs` | パターン照合、PNG 走査、寸法検証付きコピー、`DmgMount`、`import_frames` |
| `src-tauri/src/main.rs` | `set_viewport_metrics` に dpr/mobile 追加、`CaptureTarget` 化、`list_frames` / `import_frames` コマンド、合成呼び出し、opener 登録 |
| `src-tauri/frames/catalog.json` | v1 カタログ（Apple 4 + Google 9） |
| `src-tauri/frames/google/*.png`, `LICENSE`, `NOTICE` | 同梱 Pixel フレーム（スクリプト生成物） |
| `scripts/build-pixel-frames.sh` | AOSP から取得して `back` + `mask` を 1 枚に合成する開発用スクリプト |
| `src/types/frames.ts` | Rust コマンドと共有する TS 型 |
| `src/components/DeviceFramePanel.vue` | デバイス選択・状態バッジ・色選択・取り込み・シャドウトグル |
| `src/App.vue` | 状態 `rs-devices` / `rs-frame-shadow`、バリデーション、invoke 引数、パネル配置 |
| `src-tauri/tauri.conf.json`, `src-tauri/capabilities/migrated.json`, `src-tauri/Cargo.toml`, `package.json` | resources / opener 権限 / 依存 / `test` script |
| `README.md`, `AGENTS.md` | 使い方（日英）と設計制約 |

---

### Task 1: frames モジュールの骨組みと合成器（シャドウなし）

**Files:**
- Create: `src-tauri/src/frames/mod.rs`
- Create: `src-tauri/src/frames/compose.rs`
- Modify: `src-tauri/src/main.rs:16`（`use tauri::command;` の直後に `mod frames;`）

**Interfaces:**
- Produces: `frames::Rect { x, y, width, height }` + `right()` / `bottom()` / `contains(x, y)`、`frames::compose::cover_resize(&RgbaImage, w, h) -> RgbaImage`、`frames::compose::compose_frame(shot: &RgbaImage, frame: &RgbaImage, screen: Rect) -> RgbaImage`（Task 2 で `shadow: bool` を追加する）

- [ ] **Step 1: `mod.rs` を作る**

```rust
//! デバイスフレーム合成機能。カタログ（メタデータ）・保存場所・取り込み・合成器に分かれる。
//! 設計: docs/superpowers/specs/2026-08-30-device-frame-design.md

pub mod compose;

use serde::{Deserialize, Serialize};

/// 画像内の矩形（左上原点、ピクセル単位）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Rect {
    pub fn right(&self) -> u32 {
        self.x + self.width
    }

    pub fn bottom(&self) -> u32 {
        self.y + self.height
    }

    pub fn contains(&self, x: u32, y: u32) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }
}
```

- [ ] **Step 2: テストだけ書いた `compose.rs` を作る（実装はまだ空）**

```rust
//! スクリーンショットをフレーム PNG にはめ込む純関数。Tauri にも Chrome にも依存しない。

use image::imageops::{self, FilterType};
use image::RgbaImage;

use super::Rect;

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    const HOLE: Rect = Rect { x: 20, y: 30, width: 60, height: 140 };
    const BEZEL: [u8; 4] = [10, 20, 30, 255];

    fn solid(w: u32, h: u32, c: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba(c))
    }

    /// 100x200 のフレーム。HOLE の内側だけ透明（Apple / Google の公式素材と同じ構造）
    fn frame_with_hole() -> RgbaImage {
        let mut f = solid(100, 200, BEZEL);
        for y in HOLE.y..HOLE.bottom() {
            for x in HOLE.x..HOLE.right() {
                f.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
        f
    }

    #[test]
    fn screenshot_fills_hole_and_frame_covers_outside() {
        let shot = solid(60, 140, [200, 0, 0, 255]);
        let out = compose_frame(&shot, &frame_with_hole(), HOLE);
        assert_eq!(out.dimensions(), (100, 200));
        assert_eq!(out.get_pixel(50, 100).0, [200, 0, 0, 255], "画面中央はスクショ");
        assert_eq!(out.get_pixel(5, 5).0, BEZEL, "ベゼル部分はフレーム");
        assert_eq!(out.get_pixel(HOLE.x, HOLE.y).0, [200, 0, 0, 255], "画面の左上角");
    }

    #[test]
    fn wider_screenshot_is_cover_cropped_to_center() {
        // 120x140: 高さは一致、幅が 2 倍。cover なので左右 30px ずつ切り捨てられる
        let mut shot = solid(120, 140, [0, 0, 255, 255]);
        for y in 0..140 {
            for x in 0..30 {
                shot.put_pixel(x, y, Rgba([0, 255, 0, 255]));
            }
        }
        let out = compose_frame(&shot, &frame_with_hole(), HOLE);
        assert_eq!(out.dimensions(), (100, 200));
        assert_eq!(
            out.get_pixel(HOLE.x, HOLE.y + 70).0,
            [0, 0, 255, 255],
            "左端 30px の緑は切り落とされる"
        );
    }

    #[test]
    fn smaller_screenshot_is_upscaled_to_fill() {
        let shot = solid(30, 70, [0, 0, 255, 255]);
        let out = compose_frame(&shot, &frame_with_hole(), HOLE);
        assert_eq!(out.get_pixel(HOLE.right() - 1, HOLE.bottom() - 1).0, [0, 0, 255, 255]);
    }

    #[test]
    fn cover_resize_returns_exact_size_for_off_by_one_input() {
        // Pixel 9: 412 CSS px × DPR 2.625 = 1081.5 なので撮影結果が 1〜2px ずれる。これを吸収する
        let out = cover_resize(&solid(1081, 2423, [1, 2, 3, 255]), 1080, 2424);
        assert_eq!(out.dimensions(), (1080, 2424));
    }
}
```

- [ ] **Step 3: `main.rs` に `mod frames;` を追加する**

`src-tauri/src/main.rs` の `use tauri::command;`（16 行目）の直後に:

```rust
mod frames;
```

- [ ] **Step 4: テストが失敗（コンパイルエラー）することを確認**

Run: `cd src-tauri && cargo test frames::compose 2>&1 | tail -20`
Expected: `error[E0425]: cannot find function \`compose_frame\`` と `cover_resize` の未定義エラー

- [ ] **Step 5: 実装を書く（`compose.rs` の `use` 群と `#[cfg(test)]` の間に挿入）**

```rust
/// `src` を比率を保ったまま `w x h` を覆う大きさにリサイズし、中央で `w x h` に切り抜く
/// （CSS の object-fit: cover 相当）。寸法が一致していれば等倍コピー。
pub fn cover_resize(src: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    if src.width() == w && src.height() == h {
        return src.clone();
    }
    let scale = (w as f64 / src.width() as f64).max(h as f64 / src.height() as f64);
    let rw = ((src.width() as f64 * scale).ceil() as u32).max(w);
    let rh = ((src.height() as f64 * scale).ceil() as u32).max(h);
    let resized = imageops::resize(src, rw, rh, FilterType::Lanczos3);
    imageops::crop_imm(&resized, (rw - w) / 2, (rh - h) / 2, w, h).to_image()
}

/// スクショを `screen` に cover リサイズして置き、その上にフレームを重ねる。
/// フレームの画面部分は透明である前提（Apple / Google の公式素材はどちらもそう）。
/// 角丸クリップはしない: フレーム側の角が不透明でスクショの角を覆う。
pub fn compose_frame(shot: &RgbaImage, frame: &RgbaImage, screen: Rect) -> RgbaImage {
    let fitted = cover_resize(shot, screen.width, screen.height);
    let mut canvas = RgbaImage::new(frame.width(), frame.height());
    imageops::overlay(&mut canvas, &fitted, screen.x as i64, screen.y as i64);
    imageops::overlay(&mut canvas, frame, 0, 0);
    canvas
}
```

- [ ] **Step 6: テストが通ることを確認**

Run: `cd src-tauri && cargo test frames::compose 2>&1 | tail -10`
Expected: `test result: ok. 4 passed`（未使用関数の warning は出てよい）

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/frames/mod.rs src-tauri/src/frames/compose.rs src-tauri/src/main.rs
git commit -m "feat(frames): ✨ フレーム合成器（cover リサイズ + オーバーレイ）を追加"
```

---

### Task 2: ドロップシャドウ

**Files:**
- Modify: `src-tauri/src/frames/compose.rs`

**Interfaces:**
- Consumes: Task 1 の `cover_resize` / `Rect::contains`
- Produces: `ShadowParams { sigma: f32, offset_y: u32, opacity: f32, padding: u32 }` + `ShadowParams::for_frame(width, height)`、`shadow_layer(frame, screen, &ShadowParams) -> RgbaImage`、**シグネチャ変更** `compose_frame(shot, frame, screen, shadow: bool) -> RgbaImage`

- [ ] **Step 1: Task 1 のテスト 3 件の `compose_frame(...)` 呼び出しに第 4 引数 `false` を足し、シャドウのテストを追加**

`tests` モジュール末尾に追加:

```rust
    #[test]
    fn shadow_params_follow_frame_size() {
        let p = ShadowParams::for_frame(1350, 2760); // iPhone 16 Pro
        assert!((p.sigma - 20.25).abs() < 0.01);
        assert_eq!(p.offset_y, 41);
        assert_eq!(p.padding, 102); // ceil(60.75 + 41)
        assert_eq!(ShadowParams::for_frame(100, 200).padding, 8); // sigma 1.5, offset 3 → ceil(7.5)
    }

    #[test]
    fn shadow_expands_canvas_and_darkens_below_body() {
        let shot = solid(60, 140, [200, 0, 0, 255]);
        let p = ShadowParams::for_frame(100, 200);
        let out = compose_frame(&shot, &frame_with_hole(), HOLE, true);
        assert_eq!(out.dimensions(), (100 + 2 * p.padding, 200 + 2 * p.padding));

        let below = out.get_pixel(p.padding + 50, p.padding + 200 + 1);
        assert_eq!(&below.0[..3], &[0, 0, 0], "影は黒");
        assert!(below[3] > 0, "本体下端のすぐ外側に影がある (alpha={})", below[3]);
        assert!(out.get_pixel(0, 0)[3] <= 2, "四隅は透明（ガウスの裾は 3σ で 1% 未満）");
        assert_eq!(out.get_pixel(p.padding + 5, p.padding + 5).0, BEZEL, "本体はそのまま");
        assert_eq!(
            out.get_pixel(p.padding + 50, p.padding + 100).0,
            [200, 0, 0, 255],
            "画面もそのまま"
        );
    }
```

- [ ] **Step 2: 失敗を確認**

Run: `cd src-tauri && cargo test frames::compose 2>&1 | tail -20`
Expected: `ShadowParams` 未定義と引数個数のコンパイルエラー

- [ ] **Step 3: 実装（`use image::RgbaImage;` を `use image::{Rgba, RgbaImage};` に変え、`compose_frame` を置き換える）**

```rust
/// ドロップシャドウのパラメータ。フレーム寸法に対する比率で決める（spec §9.1）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowParams {
    pub sigma: f32,
    pub offset_y: u32,
    pub opacity: f32,
    pub padding: u32,
}

impl ShadowParams {
    pub fn for_frame(width: u32, height: u32) -> Self {
        let sigma = (width as f32 * 0.015).max(1.0);
        let offset_y = (height as f32 * 0.015).round() as u32;
        let padding = (3.0 * sigma + offset_y as f32).ceil() as u32;
        Self { sigma, offset_y, opacity: 0.35, padding }
    }
}

/// フレームより `padding` ずつ大きいキャンバスに、本体シルエット（フレームの不透明部 ∪ 画面矩形）を
/// 下方向に `offset_y` ずらして置き、ぼかして「黒 × opacity」にしたレイヤーを返す。
pub fn shadow_layer(frame: &RgbaImage, screen: Rect, p: &ShadowParams) -> RgbaImage {
    let (fw, fh) = frame.dimensions();
    let (cw, ch) = (fw + 2 * p.padding, fh + 2 * p.padding);

    // シルエット。画面部はフレームでは透明だが実機は塗り潰しなので矩形で埋める
    let mut mask = RgbaImage::new(cw, ch);
    for y in 0..fh {
        for x in 0..fw {
            let a = if screen.contains(x, y) { 255 } else { frame.get_pixel(x, y)[3] };
            mask.put_pixel(x + p.padding, y + p.padding + p.offset_y, Rgba([0, 0, 0, a]));
        }
    }

    // 1/4 に縮小してからぼかし、元の寸法に戻す（フルサイズの blur は 1470x3000 で数秒かかる）
    let (sw, sh) = ((cw / 4).max(1), (ch / 4).max(1));
    let small = imageops::resize(&mask, sw, sh, FilterType::Triangle);
    let blurred = imageops::blur(&small, (p.sigma / 4.0).max(0.5));
    let mut layer = imageops::resize(&blurred, cw, ch, FilterType::Triangle);
    for px in layer.pixels_mut() {
        *px = Rgba([0, 0, 0, (px[3] as f32 * p.opacity).round() as u8]);
    }
    layer
}

/// スクショを `screen` に cover リサイズして置き、その上にフレームを重ねる。
/// `shadow` が true ならキャンバスを `padding` 分広げ、影 → スクショ → フレーム の順に重ねる。
/// フレームの画面部分は透明である前提（Apple / Google の公式素材はどちらもそう）。
/// 角丸クリップはしない: フレーム側の角が不透明でスクショの角を覆う。
pub fn compose_frame(shot: &RgbaImage, frame: &RgbaImage, screen: Rect, shadow: bool) -> RgbaImage {
    let fitted = cover_resize(shot, screen.width, screen.height);
    let params = ShadowParams::for_frame(frame.width(), frame.height());
    let pad = if shadow { params.padding } else { 0 };

    let mut canvas = RgbaImage::new(frame.width() + 2 * pad, frame.height() + 2 * pad);
    if shadow {
        imageops::overlay(&mut canvas, &shadow_layer(frame, screen, &params), 0, 0);
    }
    imageops::overlay(&mut canvas, &fitted, (pad + screen.x) as i64, (pad + screen.y) as i64);
    imageops::overlay(&mut canvas, frame, pad as i64, pad as i64);
    canvas
}
```

- [ ] **Step 4: テストが通ることを確認**

Run: `cd src-tauri && cargo test frames::compose 2>&1 | tail -10`
Expected: `test result: ok. 6 passed`

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/frames/compose.rs
git commit -m "feat(frames): ✨ アプリ生成のドロップシャドウを合成器に追加"
```

---

### Task 3: カタログの型・読み込み・検証

**Files:**
- Create: `src-tauri/src/frames/catalog.rs`
- Modify: `src-tauri/src/frames/mod.rs`（`pub mod catalog;` を追加）

**Interfaces:**
- Consumes: `frames::Rect`
- Produces: `CssSpec { width: u32, height: u32, dpr: f64, mobile: bool }`、`Size { width, height }`、`Source::Bundled { file: String } | Source::Import { url: String, pattern: String }`（JSON では `"kind": "bundled" | "import"`）、`DeviceEntry { id, vendor, category, name, orientation, css, frame, screen, source }`、`parse_catalog(&str) -> Result<Vec<DeviceEntry>, String>`、`load_catalog(&Path)`、`validate(&[DeviceEntry]) -> Result<(), String>`、`find(&[DeviceEntry], id) -> Option<&DeviceEntry>`

- [ ] **Step 1: テストから書く（`catalog.rs` 全体。実装部分は空）**

```rust
//! フレームカタログ（src-tauri/frames/catalog.json）の型と検証。

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::Rect;

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[
      { "id": "google-pixel-9", "vendor": "google", "category": "phone", "name": "Pixel 9", "orientation": "portrait",
        "css": { "width": 412, "height": 923, "dpr": 2.625, "mobile": true },
        "frame": { "width": 1198, "height": 2531 },
        "screen": { "x": 55, "y": 58, "width": 1080, "height": 2424 },
        "source": { "kind": "bundled", "file": "google/pixel_9.png" } },
      { "id": "apple-iphone-16-pro", "vendor": "apple", "category": "phone", "name": "iPhone 16 Pro", "orientation": "portrait",
        "css": { "width": 402, "height": 874, "dpr": 3.0, "mobile": true },
        "frame": { "width": 1350, "height": 2760 },
        "screen": { "x": 72, "y": 69, "width": 1206, "height": 2622 },
        "source": { "kind": "import", "url": "https://example.com/Bezel-iPhone-16.dmg",
                    "pattern": "PNG/iPhone 16 Pro/iPhone 16 Pro - {variant} - Portrait.png" } }
    ]"#;

    #[test]
    fn parses_both_source_kinds() {
        let entries = parse_catalog(SAMPLE).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].source, Source::Bundled { file: "google/pixel_9.png".into() });
        assert!(matches!(&entries[1].source, Source::Import { pattern, .. } if pattern.contains("{variant}")));
        assert_eq!(find(&entries, "apple-iphone-16-pro").unwrap().css.dpr, 3.0);
        assert!(find(&entries, "nope").is_none());
    }

    #[test]
    fn rejects_duplicate_id() {
        let mut e = parse_catalog(SAMPLE).unwrap();
        e[1].id = e[0].id.clone();
        assert!(validate(&e).unwrap_err().contains("重複"));
    }

    #[test]
    fn rejects_bad_id_chars() {
        let mut e = parse_catalog(SAMPLE).unwrap();
        e[0].id = "Pixel 9".into();
        assert!(validate(&e).unwrap_err().contains("英小文字"));
    }

    #[test]
    fn rejects_screen_outside_frame() {
        let mut e = parse_catalog(SAMPLE).unwrap();
        e[0].screen.x = 200; // 200 + 1080 > 1198
        assert!(validate(&e).unwrap_err().contains("frame の外"));
    }

    #[test]
    fn rejects_pattern_without_variant() {
        let mut e = parse_catalog(SAMPLE).unwrap();
        e[1].source = Source::Import { url: "u".into(), pattern: "PNG/x.png".into() };
        assert!(validate(&e).unwrap_err().contains("{variant}"));
    }

    #[test]
    fn reports_invalid_json() {
        assert!(parse_catalog("[{").unwrap_err().starts_with("カタログの読み込みに失敗"));
    }
}
```

- [ ] **Step 2: `mod.rs` に `pub mod catalog;` を追加し、失敗を確認**

Run: `cd src-tauri && cargo test frames::catalog 2>&1 | tail -20`
Expected: `parse_catalog` / `Source` 等の未定義エラー

- [ ] **Step 3: 実装（`use super::Rect;` の直後に挿入）**

```rust
/// 撮影時の Chrome エミュレーション値
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CssSpec {
    pub width: u32,
    pub height: u32,
    pub dpr: f64,
    pub mobile: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// フレーム画像の調達元
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Source {
    /// アプリ同梱（Google Pixel）。`file` は frames/ からの相対パス
    Bundled { file: String },
    /// ユーザー取り込み（Apple）。`pattern` は `{variant}` を 1 回含むボリューム相対パス
    Import { url: String, pattern: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceEntry {
    pub id: String,
    pub vendor: String,
    pub category: String,
    pub name: String,
    pub orientation: String,
    pub css: CssSpec,
    pub frame: Size,
    pub screen: Rect,
    pub source: Source,
}

pub fn parse_catalog(json: &str) -> Result<Vec<DeviceEntry>, String> {
    let entries: Vec<DeviceEntry> =
        serde_json::from_str(json).map_err(|e| format!("カタログの読み込みに失敗: {}", e))?;
    validate(&entries)?;
    Ok(entries)
}

pub fn load_catalog(path: &Path) -> Result<Vec<DeviceEntry>, String> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| format!("カタログの読み込みに失敗: {}: {}", path.display(), e))?;
    parse_catalog(&json)
}

/// spec §5.1 の不変条件。同梱ファイルの存在と寸法はカタログ自体のテスト（Task 4）で確認する
pub fn validate(entries: &[DeviceEntry]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for e in entries {
        if !seen.insert(e.id.as_str()) {
            return Err(format!("カタログ id が重複しています: {}", e.id));
        }
        if e.id.is_empty()
            || !e.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(format!("カタログ id は英小文字・数字・ハイフンのみ: {:?}", e.id));
        }
        if e.screen.right() > e.frame.width || e.screen.bottom() > e.frame.height {
            return Err(format!("{}: screen が frame の外に出ています", e.id));
        }
        if let Source::Import { pattern, .. } = &e.source {
            if pattern.matches("{variant}").count() != 1 {
                return Err(format!(
                    "{}: pattern は {{variant}} を 1 回だけ含む必要があります",
                    e.id
                ));
            }
        }
    }
    Ok(())
}

pub fn find<'a>(entries: &'a [DeviceEntry], id: &str) -> Option<&'a DeviceEntry> {
    entries.iter().find(|e| e.id == id)
}
```

- [ ] **Step 4: テストが通ることを確認**

Run: `cd src-tauri && cargo test frames::catalog 2>&1 | tail -10`
Expected: `test result: ok. 6 passed`

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/frames/mod.rs src-tauri/src/frames/catalog.rs
git commit -m "feat(frames): ✨ フレームカタログの型と検証を追加"
```

---

### Task 4: Pixel フレーム生成スクリプト・v1 カタログ・resources 設定

**Files:**
- Create: `scripts/build-pixel-frames.sh`
- Create（スクリプト生成物）: `src-tauri/frames/google/{pixel_9,pixel_9_pro,pixel_9_pro_xl,pixel_9a,pixel_10,pixel_10_pro,pixel_10_pro_xl,pixel_10a,pixel_tablet}.png`, `src-tauri/frames/google/LICENSE`, `src-tauri/frames/google/NOTICE`
- Create: `src-tauri/frames/catalog.json`
- Modify: `src-tauri/src/frames/catalog.rs`（テスト 1 件追加）
- Modify: `src-tauri/tauri.conf.json:15`（`"resources": []` → `["frames/**/*"]`）
- Modify: `package.json`（`scripts.test` 追加）

**Interfaces:**
- Consumes: Task 3 の `load_catalog` / `Source`
- Produces: 実行時に `<resource_dir>/frames/catalog.json` と `<resource_dir>/frames/google/<id>.png` が存在する

- [ ] **Step 1: 同梱カタログの整合性テストを `catalog.rs` の `tests` に追加**

```rust
    /// 同梱カタログそのもの: 13 件、不変条件を満たし、bundled の PNG が存在して frame 寸法と一致する
    #[test]
    fn bundled_catalog_is_valid_and_bundled_pngs_match_frame_size() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("frames");
        let entries = load_catalog(&root.join("catalog.json")).expect("frames/catalog.json");
        assert_eq!(entries.len(), 13);
        for e in &entries {
            if let Source::Bundled { file } = &e.source {
                let path = root.join(file);
                let (w, h) = image::image_dimensions(&path)
                    .unwrap_or_else(|err| panic!("{}: {}", path.display(), err));
                assert_eq!((w, h), (e.frame.width, e.frame.height), "{}", e.id);
            }
        }
    }
```

- [ ] **Step 2: 失敗を確認**

Run: `cd src-tauri && cargo test bundled_catalog 2>&1 | tail -5`
Expected: `frames/catalog.json` が無いので panic（FAIL）

- [ ] **Step 3: 生成スクリプトを書く**

`scripts/build-pixel-frames.sh`:

```bash
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
```

- [ ] **Step 4: 実行して生成物の寸法を spec §3.2 の表と照合**

Run: `chmod +x scripts/build-pixel-frames.sh && scripts/build-pixel-frames.sh`
Expected（この 9 行と一致すること）:

```
pixel_9: 1198x2531 (mask at +55+58)
pixel_9_pro: 1408x2974 (mask at +60+61)
pixel_9_pro_xl: 1466x3101 (mask at +57+56)
pixel_9a: 1224x2570 (mask at +69+73)
pixel_10: 1205x2535 (mask at +59+55)
pixel_10_pro: 1410x2968 (mask at +59+60)
pixel_10_pro_xl: 1472x3111 (mask at +60+55)
pixel_10a: 1218x2553 (mask at +65+64)
pixel_tablet: 2798x1837 (mask at +119+117)
```

さらに Pixel 9 の画面部が透明のまま・カメラ穴が不透明になっていることを確認:

Run: `magick src-tauri/frames/google/pixel_9.png -format "center=%[pixel:p{599,1270}] hole=%[pixel:p{595,168}] bezel=%[pixel:p{30,600}]\n" info:`
Expected: `center=srgba(0,0,0,0)`、`hole` と `bezel` は alpha 1

- [ ] **Step 5: `src-tauri/frames/catalog.json` を書く（13 件）**

```json
[
  {
    "id": "apple-iphone-16",
    "vendor": "apple", "category": "phone", "name": "iPhone 16", "orientation": "portrait",
    "css": { "width": 393, "height": 852, "dpr": 3.0, "mobile": true },
    "frame": { "width": 1359, "height": 2736 },
    "screen": { "x": 90, "y": 90, "width": 1179, "height": 2556 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-iPhone-16.dmg",
      "pattern": "PNG/iPhone 16/iPhone 16 - {variant} - Portrait.png" }
  },
  {
    "id": "apple-iphone-16-plus",
    "vendor": "apple", "category": "phone", "name": "iPhone 16 Plus", "orientation": "portrait",
    "css": { "width": 430, "height": 932, "dpr": 3.0, "mobile": true },
    "frame": { "width": 1470, "height": 2970 },
    "screen": { "x": 90, "y": 87, "width": 1290, "height": 2796 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-iPhone-16.dmg",
      "pattern": "PNG/iPhone 16 Plus/iPhone 16 Plus - {variant} - Portrait.png" }
  },
  {
    "id": "apple-iphone-16-pro",
    "vendor": "apple", "category": "phone", "name": "iPhone 16 Pro", "orientation": "portrait",
    "css": { "width": 402, "height": 874, "dpr": 3.0, "mobile": true },
    "frame": { "width": 1350, "height": 2760 },
    "screen": { "x": 72, "y": 69, "width": 1206, "height": 2622 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-iPhone-16.dmg",
      "pattern": "PNG/iPhone 16 Pro/iPhone 16 Pro - {variant} - Portrait.png" }
  },
  {
    "id": "apple-iphone-16-pro-max",
    "vendor": "apple", "category": "phone", "name": "iPhone 16 Pro Max", "orientation": "portrait",
    "css": { "width": 440, "height": 956, "dpr": 3.0, "mobile": true },
    "frame": { "width": 1470, "height": 3000 },
    "screen": { "x": 75, "y": 66, "width": 1320, "height": 2868 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-iPhone-16.dmg",
      "pattern": "PNG/iPhone 16 Pro Max/iPhone 16 Pro Max - {variant} - Portrait.png" }
  },
  {
    "id": "google-pixel-9",
    "vendor": "google", "category": "phone", "name": "Pixel 9", "orientation": "portrait",
    "css": { "width": 412, "height": 923, "dpr": 2.625, "mobile": true },
    "frame": { "width": 1198, "height": 2531 },
    "screen": { "x": 55, "y": 58, "width": 1080, "height": 2424 },
    "source": { "kind": "bundled", "file": "google/pixel_9.png" }
  },
  {
    "id": "google-pixel-9-pro",
    "vendor": "google", "category": "phone", "name": "Pixel 9 Pro", "orientation": "portrait",
    "css": { "width": 427, "height": 952, "dpr": 3.0, "mobile": true },
    "frame": { "width": 1408, "height": 2974 },
    "screen": { "x": 60, "y": 61, "width": 1280, "height": 2856 },
    "source": { "kind": "bundled", "file": "google/pixel_9_pro.png" }
  },
  {
    "id": "google-pixel-9-pro-xl",
    "vendor": "google", "category": "phone", "name": "Pixel 9 Pro XL", "orientation": "portrait",
    "css": { "width": 448, "height": 997, "dpr": 3.0, "mobile": true },
    "frame": { "width": 1466, "height": 3101 },
    "screen": { "x": 57, "y": 56, "width": 1344, "height": 2992 },
    "source": { "kind": "bundled", "file": "google/pixel_9_pro_xl.png" }
  },
  {
    "id": "google-pixel-9a",
    "vendor": "google", "category": "phone", "name": "Pixel 9a", "orientation": "portrait",
    "css": { "width": 412, "height": 923, "dpr": 2.625, "mobile": true },
    "frame": { "width": 1224, "height": 2570 },
    "screen": { "x": 69, "y": 73, "width": 1080, "height": 2424 },
    "source": { "kind": "bundled", "file": "google/pixel_9a.png" }
  },
  {
    "id": "google-pixel-10",
    "vendor": "google", "category": "phone", "name": "Pixel 10", "orientation": "portrait",
    "css": { "width": 412, "height": 923, "dpr": 2.625, "mobile": true },
    "frame": { "width": 1205, "height": 2535 },
    "screen": { "x": 59, "y": 55, "width": 1080, "height": 2424 },
    "source": { "kind": "bundled", "file": "google/pixel_10.png" }
  },
  {
    "id": "google-pixel-10-pro",
    "vendor": "google", "category": "phone", "name": "Pixel 10 Pro", "orientation": "portrait",
    "css": { "width": 427, "height": 952, "dpr": 3.0, "mobile": true },
    "frame": { "width": 1410, "height": 2968 },
    "screen": { "x": 59, "y": 60, "width": 1280, "height": 2856 },
    "source": { "kind": "bundled", "file": "google/pixel_10_pro.png" }
  },
  {
    "id": "google-pixel-10-pro-xl",
    "vendor": "google", "category": "phone", "name": "Pixel 10 Pro XL", "orientation": "portrait",
    "css": { "width": 448, "height": 997, "dpr": 3.0, "mobile": true },
    "frame": { "width": 1472, "height": 3111 },
    "screen": { "x": 60, "y": 55, "width": 1344, "height": 2992 },
    "source": { "kind": "bundled", "file": "google/pixel_10_pro_xl.png" }
  },
  {
    "id": "google-pixel-10a",
    "vendor": "google", "category": "phone", "name": "Pixel 10a", "orientation": "portrait",
    "css": { "width": 412, "height": 923, "dpr": 2.625, "mobile": true },
    "frame": { "width": 1218, "height": 2553 },
    "screen": { "x": 65, "y": 64, "width": 1080, "height": 2424 },
    "source": { "kind": "bundled", "file": "google/pixel_10a.png" }
  },
  {
    "id": "google-pixel-tablet",
    "vendor": "google", "category": "tablet", "name": "Pixel Tablet", "orientation": "landscape",
    "css": { "width": 1280, "height": 800, "dpr": 2.0, "mobile": true },
    "frame": { "width": 2798, "height": 1837 },
    "screen": { "x": 119, "y": 117, "width": 2560, "height": 1600 },
    "source": { "kind": "bundled", "file": "google/pixel_tablet.png" }
  }
]
```

- [ ] **Step 6: `tauri.conf.json` の resources と `package.json` の test script を設定**

`src-tauri/tauri.conf.json` の `"resources": [],` を:

```json
    "resources": ["frames/**/*"],
```

`package.json` の `"scripts"` に追加（`"preview"` の次の行）:

```json
    "test": "pnpm run build && cd src-tauri && cargo test",
```

- [ ] **Step 7: テストが通ることを確認**

Run: `pnpm test 2>&1 | tail -15`
Expected: `vue-tsc` / `vite build` 成功のあと `test result: ok. 13 passed`（compose 6 + catalog 7）

- [ ] **Step 8: Commit**

```bash
git add scripts/build-pixel-frames.sh src-tauri/frames src-tauri/src/frames/catalog.rs src-tauri/tauri.conf.json package.json
git commit -m "feat(frames): ✨ Pixel フレーム生成スクリプトと v1 カタログを追加（AOSP, Apache 2.0）"
```

---

### Task 5: フレームの保存場所と状態（store.rs）

**Files:**
- Create: `src-tauri/src/frames/store.rs`
- Modify: `src-tauri/src/frames/mod.rs`（`pub mod store;` を追加）

**Interfaces:**
- Consumes: Task 3 の `DeviceEntry` / `Source`
- Produces: `Roots { bundled: PathBuf, user: PathBuf }`、`slugify(&str) -> String`、`FrameStatus { id, vendor, category, name, orientation, state: String, variants: Vec<String>, source_url: Option<String> }`（`Serialize`）、`user_variants(&Roots, id) -> Vec<String>`、`status_for(&DeviceEntry, &Roots) -> FrameStatus`、`resolve_frame_png(&DeviceEntry, variant: Option<&str>, &Roots) -> Result<PathBuf, String>`

- [ ] **Step 1: テストから書く（`store.rs` 全体。実装部分は空）**

```rust
//! フレーム画像の保存場所の解決と、UI 向けの状態（同梱 / 取り込み済み / 未取り込み）。

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::catalog::{DeviceEntry, Source};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frames::catalog::parse_catalog;

    const SAMPLE: &str = r#"[
      { "id": "google-pixel-9", "vendor": "google", "category": "phone", "name": "Pixel 9", "orientation": "portrait",
        "css": { "width": 412, "height": 923, "dpr": 2.625, "mobile": true },
        "frame": { "width": 1198, "height": 2531 },
        "screen": { "x": 55, "y": 58, "width": 1080, "height": 2424 },
        "source": { "kind": "bundled", "file": "google/pixel_9.png" } },
      { "id": "apple-iphone-16-pro", "vendor": "apple", "category": "phone", "name": "iPhone 16 Pro", "orientation": "portrait",
        "css": { "width": 402, "height": 874, "dpr": 3.0, "mobile": true },
        "frame": { "width": 1350, "height": 2760 },
        "screen": { "x": 72, "y": 69, "width": 1206, "height": 2622 },
        "source": { "kind": "import", "url": "https://example.com/Bezel-iPhone-16.dmg",
                    "pattern": "PNG/iPhone 16 Pro/iPhone 16 Pro - {variant} - Portrait.png" } }
    ]"#;

    /// テストごとに一意な一時ディレクトリ（tempfile crate は使わない方針）
    fn temp_root(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("rs-store-{}-{}-{}", tag, std::process::id(), nanos));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"png").unwrap();
    }

    fn roots(tag: &str) -> Roots {
        let base = temp_root(tag);
        Roots { bundled: base.join("bundled"), user: base.join("user") }
    }

    #[test]
    fn slugify_lowercases_and_hyphenates() {
        assert_eq!(slugify("Black Titanium"), "black-titanium");
        assert_eq!(slugify("  White  "), "white");
        assert_eq!(slugify("Ultramarine"), "ultramarine");
    }

    #[test]
    fn bundled_status_depends_on_file_presence() {
        let entries = parse_catalog(SAMPLE).unwrap();
        let r = roots("bundled");
        assert_eq!(status_for(&entries[0], &r).state, "missing");
        touch(&r.bundled.join("google/pixel_9.png"));
        let s = status_for(&entries[0], &r);
        assert_eq!(s.state, "bundled");
        assert!(s.variants.is_empty());
        assert_eq!(s.source_url, None);
    }

    #[test]
    fn import_status_lists_variants_sorted() {
        let entries = parse_catalog(SAMPLE).unwrap();
        let r = roots("import");
        assert_eq!(status_for(&entries[1], &r).state, "missing");
        touch(&r.user.join("apple-iphone-16-pro/white-titanium.png"));
        touch(&r.user.join("apple-iphone-16-pro/black-titanium.png"));
        touch(&r.user.join("apple-iphone-16-pro/notes.txt"));
        let s = status_for(&entries[1], &r);
        assert_eq!(s.state, "imported");
        assert_eq!(s.variants, vec!["black-titanium", "white-titanium"]);
        assert_eq!(s.source_url.as_deref(), Some("https://example.com/Bezel-iPhone-16.dmg"));
    }

    #[test]
    fn resolve_bundled_and_import_paths() {
        let entries = parse_catalog(SAMPLE).unwrap();
        let r = roots("resolve");
        touch(&r.bundled.join("google/pixel_9.png"));
        touch(&r.user.join("apple-iphone-16-pro/black-titanium.png"));

        assert_eq!(resolve_frame_png(&entries[0], None, &r).unwrap(), r.bundled.join("google/pixel_9.png"));
        assert_eq!(
            resolve_frame_png(&entries[1], Some("Black Titanium"), &r).unwrap(),
            r.user.join("apple-iphone-16-pro/black-titanium.png")
        );
        assert!(resolve_frame_png(&entries[1], None, &r).unwrap_err().contains("色が選択されていません"));
        assert_eq!(
            resolve_frame_png(&entries[1], Some("pink"), &r).unwrap_err(),
            "フレームが見つかりません: iPhone 16 Pro (pink)。取り込みをやり直してください"
        );
    }
}
```

- [ ] **Step 2: `mod.rs` に `pub mod store;` を追加し、失敗を確認**

Run: `cd src-tauri && cargo test frames::store 2>&1 | tail -20`
Expected: `Roots` / `slugify` 等の未定義エラー

- [ ] **Step 3: 実装（`use super::catalog::...;` の直後に挿入）**

```rust
/// フレーム画像を探す 2 つのルート
#[derive(Debug, Clone)]
pub struct Roots {
    /// 同梱: `<resource_dir>/frames`
    pub bundled: PathBuf,
    /// 取り込み: `<app_data_dir>/frames`
    pub user: PathBuf,
}

/// "Black Titanium" → "black-titanium"。UI・ファイル名ともこの表記に統一する
pub fn slugify(name: &str) -> String {
    name.split_whitespace()
        .map(|s| s.to_lowercase())
        .collect::<Vec<_>>()
        .join("-")
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FrameStatus {
    pub id: String,
    pub vendor: String,
    pub category: String,
    pub name: String,
    pub orientation: String,
    /// "bundled" | "imported" | "missing"
    pub state: String,
    /// 取り込み済みの色スラッグ（昇順）。同梱は空
    pub variants: Vec<String>,
    pub source_url: Option<String>,
}

/// `<user>/<id>/*.png` のファイル名（拡張子なし）を昇順で返す
pub fn user_variants(roots: &Roots, id: &str) -> Vec<String> {
    let dir = roots.user.join(id);
    let mut names: Vec<String> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map_or(false, |x| x.eq_ignore_ascii_case("png")))
            .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .collect(),
        Err(_) => Vec::new(),
    };
    names.sort();
    names
}

pub fn status_for(entry: &DeviceEntry, roots: &Roots) -> FrameStatus {
    let (state, variants, source_url) = match &entry.source {
        Source::Bundled { file } => {
            let state = if roots.bundled.join(file).is_file() { "bundled" } else { "missing" };
            (state, Vec::new(), None)
        }
        Source::Import { url, .. } => {
            let variants = user_variants(roots, &entry.id);
            let state = if variants.is_empty() { "missing" } else { "imported" };
            (state, variants, Some(url.clone()))
        }
    };
    FrameStatus {
        id: entry.id.clone(),
        vendor: entry.vendor.clone(),
        category: entry.category.clone(),
        name: entry.name.clone(),
        orientation: entry.orientation.clone(),
        state: state.to_string(),
        variants,
        source_url,
    }
}

/// 撮影に使うフレーム PNG のパス。無ければ spec §11 のメッセージでエラー
pub fn resolve_frame_png(
    entry: &DeviceEntry,
    variant: Option<&str>,
    roots: &Roots,
) -> Result<PathBuf, String> {
    let path = match &entry.source {
        Source::Bundled { file } => roots.bundled.join(file),
        Source::Import { .. } => {
            let v = variant.ok_or_else(|| format!("{} の色が選択されていません", entry.name))?;
            roots.user.join(&entry.id).join(format!("{}.png", slugify(v)))
        }
    };
    if path.is_file() {
        Ok(path)
    } else {
        let label = variant.map(slugify).unwrap_or_else(|| "同梱".to_string());
        Err(format!(
            "フレームが見つかりません: {} ({})。取り込みをやり直してください",
            entry.name, label
        ))
    }
}
```

- [ ] **Step 4: テストが通ることを確認**

Run: `cd src-tauri && cargo test frames::store 2>&1 | tail -10`
Expected: `test result: ok. 4 passed`

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/frames/mod.rs src-tauri/src/frames/store.rs
git commit -m "feat(frames): ✨ フレームの保存場所解決と状態一覧を追加"
```

---

### Task 6: キャプチャループの CaptureTarget 化（挙動変更なし）

**Files:**
- Modify: `src-tauri/src/main.rs:21-43`（`set_viewport_metrics`）、`:227`（`capture_fullpage_expanded_viewport` 内の呼び出し）、`:349-392`（`capture_screenshots` 冒頭とループ入口）、`:584-597`（メトリクス再適用とファイル名）

**Interfaces:**
- Produces: `set_viewport_metrics(tab, width, height, device_scale_factor: f64, mobile: bool)`、`struct CaptureTarget { width, height, dpr, mobile, label, frame: Option<FrameJob> }`、`struct FrameJob { frame_png: PathBuf, screen: frames::Rect, shadow: bool }`。ループ変数は `for target in targets`。Task 7 が `frame: Some(..)` を使う

- [ ] **Step 1: `set_viewport_metrics` に `device_scale_factor` と `mobile` を追加**

`src-tauri/src/main.rs:21-31` を次に置き換える（他のフィールドはそのまま）:

```rust
fn set_viewport_metrics(
    tab: &headless_chrome::browser::tab::Tab,
    width: u32,
    height: u32,
    device_scale_factor: f64,
    mobile: bool,
) -> Result<(), String> {
    tab.call_method(Emulation::SetDeviceMetricsOverride {
        width,
        height,
        device_scale_factor,
        mobile,
```

`capture_fullpage_expanded_viewport` 内（227 行目付近）の呼び出しは幅指定専用なので固定値を渡す:

```rust
    set_viewport_metrics(tab, width_px, height_px, 1.0, false)?;
```

- [ ] **Step 2: `CaptureTarget` / `FrameJob` を `capture_screenshots` の直前（`#[command] fn abort_capture` の後）に追加**

```rust
/// フレーム合成の指示。Some のターゲットは viewport 固定・PNG 固定で、保存前に合成する
struct FrameJob {
    frame_png: PathBuf,
    screen: frames::Rect,
    shadow: bool,
}

/// 1 回のブラウザ起動で撮る対象。幅指定は dpr 1.0 / mobile false、デバイスはカタログの値
struct CaptureTarget {
    width: u32,
    height: u32,
    dpr: f64,
    mobile: bool,
    /// ファイル名用ラベル。幅指定は従来どおり "1440px" / "1440x810"
    label: String,
    frame: Option<FrameJob>,
}
```

- [ ] **Step 3: ループ入口をターゲット列挙に変える**

`capture_screenshots` 内の

```rust
    let capture_height = viewport_height.unwrap_or(VIEWPORT_HEIGHT).max(1);

    for w in widths {
```

を次に置き換える:

```rust
    let capture_height = viewport_height.unwrap_or(VIEWPORT_HEIGHT).max(1);
    let targets: Vec<CaptureTarget> = widths
        .iter()
        .map(|&w| CaptureTarget {
            width: w,
            height: capture_height,
            dpr: 1.0,
            mobile: false,
            label: if viewport_height.is_some() {
                format!("{}x{}", w, capture_height)
            } else {
                format!("{}px", w)
            },
            frame: None,
        })
        .collect();

    for target in targets {
        // 既存のループ本体は w / capture_height を参照しているので、同名で束縛し直して差分を最小にする
        let w = target.width;
        let capture_height = target.height;
```

- [ ] **Step 4: ループ内の 2 か所の `set_viewport_metrics(&tab, w, capture_height)?;`（旧 385 行目と 584 行目）を置き換える**

```rust
        set_viewport_metrics(&tab, w, capture_height, target.dpr, target.mobile)?;
```

- [ ] **Step 5: ファイル名ラベルをターゲットから取る**

旧 588-592 行目の

```rust
        let size_label = if viewport_height.is_some() {
            format!("{}x{}", w, capture_height)
        } else {
            format!("{}px", w)
        };
```

を次に置き換える（後続の `format!("capture_{}_{}.gif", size_label, mode)` はそのまま動く）:

```rust
        let size_label = &target.label;
```

- [ ] **Step 6: ビルドとテストが通ることを確認**

Run: `cd src-tauri && cargo build 2>&1 | grep -E "^(warning: unused|error)" ; cargo test 2>&1 | tail -3`
Expected: `error` なし（`FrameJob` / `frame` の未使用 warning は Task 7 で消える）、`test result: ok. 17 passed`

- [ ] **Step 7: 手動確認（GUI。人が行う）— 既存出力が変わらないこと**

`pnpm run tauri:dev` で `https://example.com` を幅 375 / 1024、fullpage、PNG で撮影し、`~/Downloads/capture_375px_fullpage.png` と `capture_1024px_fullpage.png` が従来どおり生成されることを確認。`sips -g pixelWidth -g pixelHeight` で幅が 375 / 1024 であること。

- [ ] **Step 8: Commit**

```bash
git add src-tauri/src/main.rs
git commit -m "refactor: ♻️ キャプチャループを CaptureTarget の列挙に一般化（挙動変更なし）"
```

---

### Task 7: デバイスターゲットの撮影・合成と list_frames コマンド

**Files:**
- Modify: `src-tauri/src/main.rs`（`use` 追加、ヘルパー 3 つ、`capture_screenshots` の引数と targets 追加、ループ内の mode 上書き・ファイル名・保存前合成、`generate_handler!`）

**Interfaces:**
- Consumes: Task 3 `catalog::{load_catalog, find, DeviceEntry}`、Task 5 `store::{Roots, status_for, resolve_frame_png, slugify, FrameStatus}`、Task 2 `compose::compose_frame`
- Produces: Tauri コマンド `list_frames() -> Vec<FrameStatus>`、`capture_screenshots` の追加引数 `devices: Vec<DeviceSelection { id: String, variant: Option<String> }>`（JS からは `devices: [{ id, variant }]`）と `frame_shadow: bool`（JS からは `frameShadow`）

- [ ] **Step 1: `use` を追加**

`mod frames;` の直後に:

```rust
use frames::{catalog, compose, store};
use tauri::Manager;
```

- [ ] **Step 2: ルート解決・カタログ読込・`list_frames` を `abort_capture` の直後に追加**

```rust
/// 同梱フレーム（resource）と取り込みフレーム（app_data）のルート。
/// `tauri dev` で resources が解決できない場合は開発ビルドに限り src-tauri/frames を直接読む。
fn frame_roots(app: &tauri::AppHandle) -> Result<store::Roots, String> {
    let bundled = app
        .path()
        .resolve("frames", tauri::path::BaseDirectory::Resource)
        .map_err(|e| e.to_string())?;
    #[cfg(debug_assertions)]
    let bundled = if bundled.join("catalog.json").is_file() {
        bundled
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("frames")
    };
    let user = app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("frames");
    Ok(store::Roots { bundled, user })
}

fn load_catalog(roots: &store::Roots) -> Result<Vec<catalog::DeviceEntry>, String> {
    catalog::load_catalog(&roots.bundled.join("catalog.json"))
}

#[command]
fn list_frames(app: tauri::AppHandle) -> Result<Vec<store::FrameStatus>, String> {
    let roots = frame_roots(&app)?;
    let entries = load_catalog(&roots)?;
    Ok(entries.iter().map(|e| store::status_for(e, &roots)).collect())
}

#[derive(serde::Deserialize)]
struct DeviceSelection {
    id: String,
    variant: Option<String>,
}

/// 撮影した PNG をフレームに合成して PNG バイト列を返す
fn compose_png(shot_png: &[u8], job: &FrameJob) -> Result<Vec<u8>, String> {
    let shot = image::load_from_memory(shot_png)
        .map_err(|e| format!("スクリーンショットの読み込みに失敗: {}", e))?
        .to_rgba8();
    let frame = image::open(&job.frame_png)
        .map_err(|e| format!("フレーム画像の読み込みに失敗 {}: {}", job.frame_png.display(), e))?
        .to_rgba8();
    let out = compose::compose_frame(&shot, &frame, job.screen, job.shadow);
    let mut buf = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(out)
        .write_to(&mut buf, ImageOutputFormat::Png)
        .map_err(|e| format!("PNG エンコードに失敗: {}", e))?;
    Ok(buf.into_inner())
}
```

- [ ] **Step 3: `capture_screenshots` のシグネチャに `app` と 2 引数を追加**

```rust
#[command]
fn capture_screenshots(
    app: tauri::AppHandle,
    url: String,
    widths: Vec<u32>,
    mode: String,
    _format: String,
    selector: String,
    save_dir: String,
    duration: u32,
    delay: u32,
    manual_interaction: bool,
    viewport_height: Option<u32>,
    devices: Vec<DeviceSelection>,
    frame_shadow: bool,
) -> Result<(), String> {
```

- [ ] **Step 4: 幅ターゲットの直後にデバイスターゲットを足す**

Task 6 で入れた `let targets: Vec<CaptureTarget> = ...collect();` を `let mut targets` にし、その直後（`for target in targets {` の前）に:

```rust
    if !devices.is_empty() {
        if duration > 0 {
            return Err("デバイスフレームは PNG 出力のみ対応しています".to_string());
        }
        let roots = frame_roots(&app)?;
        let entries = load_catalog(&roots)?;
        for sel in &devices {
            let entry = catalog::find(&entries, &sel.id)
                .ok_or_else(|| format!("カタログに無いデバイスです: {}", sel.id))?;
            // フレームが無ければここで止める（撮影を始めない）
            let frame_png = store::resolve_frame_png(entry, sel.variant.as_deref(), &roots)?;
            let label = match &sel.variant {
                Some(v) => format!("{}_{}", entry.id, store::slugify(v)),
                None => entry.id.clone(),
            };
            targets.push(CaptureTarget {
                width: entry.css.width,
                height: entry.css.height,
                dpr: entry.css.dpr,
                mobile: entry.css.mobile,
                label,
                frame: Some(FrameJob {
                    frame_png,
                    screen: entry.screen,
                    shadow: frame_shadow,
                }),
            });
        }
    }
```

- [ ] **Step 5: ループ冒頭でデバイスターゲットの mode を viewport に固定**

`let capture_height = target.height;` の直後に追加（以降のループ本体の `mode` はすべてこの束縛を参照する）:

```rust
        // デバイスターゲットは常に viewport 撮影（spec §8）
        let mode = if target.frame.is_some() {
            "viewport".to_string()
        } else {
            mode.clone()
        };
```

- [ ] **Step 6: ファイル名の分岐を置き換える**

```rust
        let file_name = if duration > 0 {
            format!("capture_{}_{}.gif", size_label, mode)
        } else {
            format!("capture_{}_{}.png", size_label, mode)
        };
```

を:

```rust
        let file_name = match &target.frame {
            Some(job) if job.shadow => format!("capture_{}_framed-shadow.png", size_label),
            Some(_) => format!("capture_{}_framed.png", size_label),
            None if duration > 0 => format!("capture_{}_{}.gif", size_label, mode),
            None => format!("capture_{}_{}.png", size_label, mode),
        };
```

- [ ] **Step 7: PNG 保存の直前で合成**

PNG 分岐末尾の

```rust
            std::fs::write(&dst, png_data)
                .map_err(|e| format!("Failed to save image at {:?}: {}", dst, e))?;
```

を:

```rust
            let png_data = match &target.frame {
                Some(job) => compose_png(&png_data, job)?,
                None => png_data,
            };
            std::fs::write(&dst, png_data)
                .map_err(|e| format!("Failed to save image at {:?}: {}", dst, e))?;
```

- [ ] **Step 8: `generate_handler!` に `list_frames` を登録**

```rust
        .invoke_handler(tauri::generate_handler![
            get_default_save_dir,
            select_element,
            capture_screenshots,
            abort_capture,
            list_frames
        ])
```

- [ ] **Step 9: ビルドとテスト**

Run: `cd src-tauri && cargo build 2>&1 | grep -E "^error" ; cargo test 2>&1 | tail -3`
Expected: `error` なし、`test result: ok. 17 passed`

- [ ] **Step 10: Commit**

```bash
git add src-tauri/src/main.rs
git commit -m "feat(frames): ✨ デバイスプリセットでの撮影とフレーム合成、list_frames コマンドを追加"
```

---

### Task 8: フロントエンド — DeviceFramePanel（選択・シャドウ）と App.vue 連携

**Files:**
- Create: `src/types/frames.ts`
- Create: `src/components/DeviceFramePanel.vue`
- Modify: `src/App.vue:1-5`（import）、`:36`（state 追加）、`:142-145`（バリデーション）、`:153-163`（invoke 引数）、`:388`（幅セクションの直後にパネル配置）

**Interfaces:**
- Consumes: Task 7 の `list_frames` と `capture_screenshots(devices, frameShadow)`
- Produces: `DeviceFramePanel` の `v-model:selected: DeviceSelection[]`、`v-model:shadow: boolean`、`disabled: boolean`、`@status(message: string)`、`expose refresh()`

- [ ] **Step 1: 型を書く `src/types/frames.ts`**

```ts
/** Rust 側 frames::store::FrameStatus と同じ形（invoke('list_frames') の戻り値） */
export interface FrameStatus {
  id: string
  vendor: 'apple' | 'google'
  category: 'phone' | 'tablet'
  name: string
  orientation: 'portrait' | 'landscape'
  state: 'bundled' | 'imported' | 'missing'
  /** 取り込み済みの色スラッグ（例 "black-titanium"）。同梱は空 */
  variants: string[]
  source_url: string | null
}

/** capture_screenshots の devices 引数の要素。同梱デバイスは variant null */
export interface DeviceSelection {
  id: string
  variant: string | null
}

/** invoke('import_frames') の戻り値（Task 10 で使う） */
export interface ImportReport {
  imported: { id: string; variant: string }[]
  skipped: { file: string; reason: string }[]
}
```

- [ ] **Step 2: パネルを書く `src/components/DeviceFramePanel.vue`**

```vue
<script setup lang="ts">
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import type { DeviceSelection, FrameStatus } from '../types/frames'

defineProps<{ disabled: boolean }>()
const selected = defineModel<DeviceSelection[]>('selected', { required: true })
const shadow = defineModel<boolean>('shadow', { required: true })
const emit = defineEmits<{ status: [message: string] }>()

const frames = ref<FrameStatus[]>([])
const vendorLabels: Record<FrameStatus['vendor'], string> = { apple: 'Apple', google: 'Google' }
const stateLabels: Record<FrameStatus['state'], string> = {
  bundled: '同梱',
  imported: '取り込み済み',
  missing: '未取り込み',
}

const groups = computed(() =>
  (['apple', 'google'] as const)
    .map(vendor => ({
      vendor,
      label: vendorLabels[vendor],
      items: frames.value.filter(f => f.vendor === vendor),
    }))
    .filter(g => g.items.length > 0)
)

/** list_frames の結果に合わせて選択を整える: 無い/未取り込みは外し、無い色は先頭の色にする */
const reconcile = () => {
  selected.value = selected.value.flatMap(sel => {
    const f = frames.value.find(x => x.id === sel.id)
    if (!f || f.state === 'missing') return []
    if (f.state === 'bundled') return [{ id: sel.id, variant: null }]
    const variant = sel.variant && f.variants.includes(sel.variant) ? sel.variant : f.variants[0]
    return [{ id: sel.id, variant }]
  })
}

const refresh = async () => {
  try {
    frames.value = await invoke<FrameStatus[]>('list_frames')
    reconcile()
  } catch (e) {
    emit('status', `フレーム一覧の取得に失敗: ${e}`)
  }
}

const isSelected = (id: string) => selected.value.some(s => s.id === id)

const toggle = (f: FrameStatus) => {
  if (f.state === 'missing') return
  if (isSelected(f.id)) {
    selected.value = selected.value.filter(s => s.id !== f.id)
  } else {
    selected.value = [
      ...selected.value,
      { id: f.id, variant: f.state === 'imported' ? f.variants[0] : null },
    ]
  }
}

onMounted(refresh)
defineExpose({ refresh })
</script>

<template>
  <section
    class="bg-white dark:bg-gray-800 p-6 rounded-xl shadow-sm border border-gray-100 dark:border-gray-700"
    :class="{ 'opacity-45 pointer-events-none': disabled }"
  >
    <div class="flex justify-between items-center mb-1">
      <h2 class="text-sm font-medium">デバイスフレーム</h2>
      <label class="flex items-center gap-2 cursor-pointer text-sm">
        <input type="checkbox" v-model="shadow" class="text-blue-500 rounded" />
        ドロップシャドウ
      </label>
    </div>
    <p class="text-xs text-gray-500 dark:text-gray-400 mb-4">
      選んだ端末の解像度で撮影し、フレームにはめ込んだ PNG を保存します（PNG 出力のみ。GIF では使えません）。
    </p>

    <div v-for="g in groups" :key="g.vendor" class="mb-4 last:mb-0">
      <h3 class="text-xs font-bold text-gray-500 dark:text-gray-400 uppercase tracking-wide mb-2">{{ g.label }}</h3>
      <div class="flex flex-wrap gap-3">
        <label
          v-for="f in g.items"
          :key="f.id"
          class="flex items-center gap-2 px-3 py-2 border border-gray-200 dark:border-gray-700 rounded-md transition-colors"
          :class="[
            f.state === 'missing'
              ? 'opacity-60 cursor-not-allowed'
              : 'cursor-pointer hover:bg-gray-50 dark:hover:bg-gray-700',
            { 'border-blue-400 bg-blue-50 dark:bg-blue-900/20': isSelected(f.id) },
          ]"
        >
          <input
            type="checkbox"
            :checked="isSelected(f.id)"
            :disabled="f.state === 'missing'"
            @change="toggle(f)"
            class="text-blue-500 rounded"
          />
          <span class="text-sm">{{ f.name }}</span>
          <span
            class="text-xs px-1.5 py-0.5 rounded"
            :class="
              f.state === 'missing'
                ? 'bg-gray-200 dark:bg-gray-700 text-gray-600 dark:text-gray-300'
                : 'bg-green-100 dark:bg-green-900/40 text-green-700 dark:text-green-300'
            "
          >
            {{ stateLabels[f.state] }}
          </span>
        </label>
      </div>
    </div>
  </section>
</template>
```

- [ ] **Step 3: `App.vue` を繋ぐ**

`<script setup>` 冒頭の import に追加:

```ts
import DeviceFramePanel from './components/DeviceFramePanel.vue'
import type { DeviceSelection } from './types/frames'
```

`const customRatioH = useStorage('rs-custom-ratio-h', 4)` の直後に:

```ts
const selectedDevices = useStorage<DeviceSelection[]>('rs-devices', [])
const frameShadow = useStorage('rs-frame-shadow', false)
```

`captureScreenshots` 内のバリデーション

```ts
  if (selectedWidths.value.length === 0) {
    statusMessage.value = "キャプチャする幅を一つ以上選択してください。"
    return
  }
```

を:

```ts
  const devices = outputFormat.value === 'gif' ? [] : selectedDevices.value
  if (selectedWidths.value.length === 0 && devices.length === 0) {
    statusMessage.value = "キャプチャする幅かデバイスを一つ以上選択してください。"
    return
  }
```

`invoke('capture_screenshots', { ... viewportHeight: viewportHeight.value })` の末尾 2 行を:

```ts
      viewportHeight: viewportHeight.value,
      devices,
      frameShadow: frameShadow.value
    })
```

テンプレートの `<!-- Save Directory -->` の直前に:

```vue
      <!-- Device Frames -->
      <DeviceFramePanel
        v-model:selected="selectedDevices"
        v-model:shadow="frameShadow"
        :disabled="outputFormat === 'gif'"
        @status="statusMessage = $event"
      />

```

- [ ] **Step 4: 型チェックとビルド**

Run: `pnpm run build 2>&1 | tail -5`
Expected: `vue-tsc` エラーなし、`vite build` 成功

- [ ] **Step 5: 手動 E2E（GUI。人が行う）— Google だけで end-to-end**

`pnpm run tauri:dev` で:
1. パネルに Apple 4 件（未取り込み・チェック不可）と Google 9 件（同梱）が並ぶ
2. 幅を全解除、Pixel 9 と Pixel Tablet をチェック、シャドウ OFF、PNG、`https://example.com` で実行
3. `~/Downloads/capture_google-pixel-9_framed.png`（1198×2531）と `capture_google-pixel-tablet_framed.png`（2798×1837）ができ、画面がフレームに収まり角がベゼルで覆われている（Preview で目視）
4. シャドウ ON で再実行 → `capture_google-pixel-9_framed-shadow.png` が 1382×2715（padding 92）で本体の下に影
5. 出力を GIF に切り替えるとパネルが薄くなり、実行してもデバイス分は撮られない
6. 幅 375 も一緒に選ぶと `capture_375px_fullpage.png` が従来どおり出る

寸法確認: `sips -g pixelWidth -g pixelHeight ~/Downloads/capture_google-pixel-9_framed*.png`

- [ ] **Step 6: Commit**

```bash
git add src/types/frames.ts src/components/DeviceFramePanel.vue src/App.vue
git commit -m "feat(ui): ✨ デバイスフレーム選択パネルとシャドウ切替を追加"
```

---

### Task 9: 取り込みロジック（import.rs — パターン照合・走査・寸法検証付きコピー）

**Files:**
- Create: `src-tauri/src/frames/import.rs`
- Modify: `src-tauri/src/frames/mod.rs`（`pub mod import;` を追加）

**Interfaces:**
- Consumes: Task 3 `DeviceEntry` / `Source`、Task 5 `slugify`
- Produces: `PatternParts { prefix, suffix }`、`split_pattern(&str) -> Option<PatternParts>`、`match_variant(&PatternParts, candidate: &str, by_file_name: bool) -> Option<String>`、`scan_pngs(&Path) -> Vec<PathBuf>`、`ImportedFrame { id, variant }` / `SkippedFile { file, reason }` / `ImportReport { imported, skipped }`（`Serialize`）、`import_pngs(files: &[PathBuf], root: &Path, by_file_name: bool, entries: &[DeviceEntry], user_dir: &Path) -> Result<ImportReport, String>`

- [ ] **Step 1: テストから書く（`import.rs` 全体。実装部分は空）**

```rust
//! Apple 公式 DMG / フォルダ / PNG からフレーム画像を取り込む。

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::catalog::{DeviceEntry, Source};
use super::store::slugify;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frames::catalog::{CssSpec, Size};
    use crate::frames::Rect;
    use image::RgbaImage;

    fn entry(id: &str, name: &str, pattern: &str, w: u32, h: u32) -> DeviceEntry {
        DeviceEntry {
            id: id.into(),
            vendor: "apple".into(),
            category: "phone".into(),
            name: name.into(),
            orientation: "portrait".into(),
            css: CssSpec { width: 402, height: 874, dpr: 3.0, mobile: true },
            frame: Size { width: w, height: h },
            screen: Rect { x: 1, y: 1, width: w - 2, height: h - 2 },
            source: Source::Import { url: "https://example.com/x.dmg".into(), pattern: pattern.into() },
        }
    }

    fn pro() -> DeviceEntry {
        entry("apple-iphone-16-pro", "iPhone 16 Pro", "PNG/iPhone 16 Pro/iPhone 16 Pro - {variant} - Portrait.png", 100, 200)
    }

    fn temp_root(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("rs-import-{}-{}-{}", tag, std::process::id(), nanos));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_png(path: &Path, w: u32, h: u32) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        RgbaImage::new(w, h).save(path).unwrap();
    }

    #[test]
    fn split_pattern_splits_at_single_placeholder() {
        let p = split_pattern("PNG/iPhone 16 Pro/iPhone 16 Pro - {variant} - Portrait.png").unwrap();
        assert_eq!(p.prefix, "PNG/iPhone 16 Pro/iPhone 16 Pro - ");
        assert_eq!(p.suffix, " - Portrait.png");
        assert!(split_pattern("PNG/x.png").is_none());
        assert!(split_pattern("{variant}/{variant}.png").is_none());
    }

    #[test]
    fn match_variant_by_path_and_by_file_name() {
        let p = split_pattern("PNG/iPhone 16 Pro/iPhone 16 Pro - {variant} - Portrait.png").unwrap();
        assert_eq!(
            match_variant(&p, "PNG/iPhone 16 Pro/iPhone 16 Pro - Black Titanium - Portrait.png", false),
            Some("Black Titanium".to_string())
        );
        assert_eq!(match_variant(&p, "iPhone 16 Pro - Black Titanium - Portrait.png", true), Some("Black Titanium".to_string()));
        assert_eq!(match_variant(&p, "PNG/iPhone 16 Pro Max/iPhone 16 Pro Max - Black Titanium - Portrait.png", false), None);
        assert_eq!(match_variant(&p, "iPhone 16 Pro Max - Black Titanium - Portrait.png", true), None);
        assert_eq!(match_variant(&p, "iPhone 16 Pro - Black Titanium - Landscape.png", true), None);
        assert_eq!(match_variant(&p, "iPhone 16 Pro -  - Portrait.png", true), None, "色名が空");
    }

    #[test]
    fn import_copies_matching_png_as_slug() {
        let root = temp_root("copy");
        let user = root.join("user");
        write_png(&root.join("PNG/iPhone 16 Pro/iPhone 16 Pro - Black Titanium - Portrait.png"), 100, 200);
        write_png(&root.join("PNG/iPhone 16 Pro Max/iPhone 16 Pro Max - Black Titanium - Portrait.png"), 110, 220);

        let files = scan_pngs(&root);
        let report = import_pngs(&files, &root, false, &[pro()], &user).unwrap();

        assert_eq!(report.imported, vec![ImportedFrame { id: "apple-iphone-16-pro".into(), variant: "black-titanium".into() }]);
        assert!(report.skipped.is_empty(), "Pro Max はどのエントリにも合わないので黙って無視");
        assert!(user.join("apple-iphone-16-pro/black-titanium.png").is_file());
    }

    #[test]
    fn import_skips_dimension_mismatch_and_hidden_files() {
        let root = temp_root("skip");
        let user = root.join("user");
        write_png(&root.join("PNG/iPhone 16 Pro/iPhone 16 Pro - White Titanium - Portrait.png"), 90, 200);
        write_png(&root.join("PNG/iPhone 16 Pro/._iPhone 16 Pro - Black Titanium - Portrait.png"), 100, 200);

        let files = scan_pngs(&root);
        assert_eq!(files.len(), 1, "._ ファイルは走査対象外");
        let report = import_pngs(&files, &root, false, &[pro()], &user).unwrap();

        assert!(report.imported.is_empty());
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].reason, "寸法が不一致 (期待 100x200, 実際 90x200)");
        assert!(!user.join("apple-iphone-16-pro").exists());
    }

    #[test]
    fn import_by_file_name_from_flat_folder() {
        let root = temp_root("flat");
        let user = root.join("user");
        write_png(&root.join("iPhone 16 Pro - Natural Titanium - Portrait.png"), 100, 200);
        let files = scan_pngs(&root);
        let report = import_pngs(&files, &root, true, &[pro()], &user).unwrap();
        assert_eq!(report.imported[0].variant, "natural-titanium");
    }
}
```

- [ ] **Step 2: `mod.rs` に `pub mod import;` を追加し、失敗を確認**

Run: `cd src-tauri && cargo test frames::import 2>&1 | tail -20`
Expected: `split_pattern` 等の未定義エラー

- [ ] **Step 3: 実装（`use super::store::slugify;` の直後に挿入）**

```rust
/// `pattern` を `{variant}` の前後で分けたもの
#[derive(Debug, Clone, PartialEq)]
pub struct PatternParts {
    pub prefix: String,
    pub suffix: String,
}

/// `{variant}` をちょうど 1 回含むときだけ Some
pub fn split_pattern(pattern: &str) -> Option<PatternParts> {
    let mut it = pattern.splitn(2, "{variant}");
    let prefix = it.next()?.to_string();
    let suffix = it.next()?.to_string();
    if suffix.contains("{variant}") {
        return None;
    }
    Some(PatternParts { prefix, suffix })
}

/// DMG: ボリューム相対パス全体で照合。フォルダ / 単一 PNG: ファイル名を prefix の最後のパス成分以降と照合。
/// 間の文字列（色名）が空、または `/` を含むものは不一致
pub fn match_variant(parts: &PatternParts, candidate: &str, by_file_name: bool) -> Option<String> {
    let prefix: &str = if by_file_name {
        parts.prefix.rsplit('/').next().unwrap_or(&parts.prefix)
    } else {
        &parts.prefix
    };
    let rest = candidate.strip_prefix(prefix)?;
    let middle = rest.strip_suffix(parts.suffix.as_str())?;
    if middle.is_empty() || middle.contains('/') {
        return None;
    }
    Some(middle.to_string())
}

/// `root` 以下の PNG を再帰的に集める（`.` で始まる名前 = macOS の `._` リソースフォークや `.fseventsd` は除外）
pub fn scan_pngs(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let rd = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().map_or(false, |x| x.eq_ignore_ascii_case("png")) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportedFrame {
    pub id: String,
    pub variant: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SkippedFile {
    pub file: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ImportReport {
    pub imported: Vec<ImportedFrame>,
    pub skipped: Vec<SkippedFile>,
}

/// `files` をカタログの import エントリに照合し、寸法が合うものを `<user_dir>/<id>/<variant-slug>.png` にコピーする。
/// どのエントリにも合わないファイルは黙って無視する（DMG には PSD 等も入っている）。
pub fn import_pngs(
    files: &[PathBuf],
    root: &Path,
    by_file_name: bool,
    entries: &[DeviceEntry],
    user_dir: &Path,
) -> Result<ImportReport, String> {
    let patterns: Vec<(&DeviceEntry, PatternParts)> = entries
        .iter()
        .filter_map(|e| match &e.source {
            Source::Import { pattern, .. } => split_pattern(pattern).map(|p| (e, p)),
            Source::Bundled { .. } => None,
        })
        .collect();

    let mut report = ImportReport::default();
    for file in files {
        let candidate = if by_file_name {
            file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        } else {
            file.strip_prefix(root)
                .map_err(|e| e.to_string())?
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        };

        let matched = patterns
            .iter()
            .find_map(|(e, p)| match_variant(p, &candidate, by_file_name).map(|v| (*e, v)));
        let (entry, variant) = match matched {
            Some(m) => m,
            None => continue,
        };

        let (w, h) = match image::image_dimensions(file) {
            Ok(d) => d,
            Err(e) => {
                report.skipped.push(SkippedFile { file: candidate, reason: format!("画像として読めません: {}", e) });
                continue;
            }
        };
        if (w, h) != (entry.frame.width, entry.frame.height) {
            report.skipped.push(SkippedFile {
                file: candidate,
                reason: format!(
                    "寸法が不一致 (期待 {}x{}, 実際 {}x{})",
                    entry.frame.width, entry.frame.height, w, h
                ),
            });
            continue;
        }

        let slug = slugify(&variant);
        let dest_dir = user_dir.join(&entry.id);
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("保存先を作成できません {}: {}", dest_dir.display(), e))?;
        let dest = dest_dir.join(format!("{}.png", slug));
        std::fs::copy(file, &dest)
            .map_err(|e| format!("コピーに失敗 {} → {}: {}", file.display(), dest.display(), e))?;
        report.imported.push(ImportedFrame { id: entry.id.clone(), variant: slug });
    }
    Ok(report)
}
```

- [ ] **Step 4: テストが通ることを確認**

Run: `cd src-tauri && cargo test frames::import 2>&1 | tail -10`
Expected: `test result: ok. 5 passed`

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/frames/mod.rs src-tauri/src/frames/import.rs
git commit -m "feat(frames): ✨ フレーム PNG の照合・寸法検証・取り込みコピーを追加"
```

---

### Task 10: DMG マウント・import_frames コマンド・opener プラグイン

**Files:**
- Modify: `src-tauri/src/frames/import.rs`（`DmgMount` と `import_frames` を追加）
- Modify: `src-tauri/src/main.rs`（`import_frames` コマンド、`use frames::import`、opener 登録、`generate_handler!`）
- Modify: `src-tauri/Cargo.toml`（`tauri-plugin-opener = "2"`）
- Modify: `src-tauri/capabilities/migrated.json`（`"opener:default"`）
- Modify: `package.json`（`@tauri-apps/plugin-opener`）

**Interfaces:**
- Consumes: Task 9 `scan_pngs` / `import_pngs`、Task 7 `frame_roots` / `load_catalog`
- Produces: `import::DmgMount::attach(&Path) -> Result<DmgMount, String>`（Drop で detach）、`import::import_frames(path: &Path, entries, user_dir) -> Result<ImportReport, String>`、Tauri コマンド `import_frames(path: String) -> ImportReport`、フロントから `openUrl()` が使える

- [ ] **Step 1: テストを `import.rs` の `tests` に追加**

```rust
    #[test]
    fn import_frames_dispatches_on_folder_file_and_missing() {
        let root = temp_root("dispatch");
        let user = root.join("user");
        let png = root.join("iPhone 16 Pro - Desert Titanium - Portrait.png");
        write_png(&png, 100, 200);

        let by_dir = import_frames(&root, &[pro()], &user).unwrap();
        assert_eq!(by_dir.imported[0].variant, "desert-titanium");

        let by_file = import_frames(&png, &[pro()], &user).unwrap();
        assert_eq!(by_file.imported.len(), 1);

        let err = import_frames(&root.join("nope.dmg"), &[pro()], &user).unwrap_err();
        assert!(
            err.contains("DMG のマウントに失敗") || err.contains("hdiutil を起動できません"),
            "{}",
            err
        );
    }
```

- [ ] **Step 2: 失敗を確認**

Run: `cd src-tauri && cargo test import_frames_dispatches 2>&1 | tail -5`
Expected: `import_frames` 未定義エラー

- [ ] **Step 3: `DmgMount` と `import_frames` を `import_pngs` の直後に追加**

```rust
/// `hdiutil attach` したボリューム。Drop で必ず detach する（エラー経路含む）
pub struct DmgMount {
    mountpoint: PathBuf,
}

impl DmgMount {
    pub fn attach(dmg: &Path) -> Result<Self, String> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let mountpoint =
            std::env::temp_dir().join(format!("responsiveshot-dmg-{}-{}", std::process::id(), nanos));
        std::fs::create_dir_all(&mountpoint).map_err(|e| e.to_string())?;

        let mut child = Command::new("hdiutil")
            .args(["attach", "-nobrowse", "-readonly", "-mountpoint"])
            .arg(&mountpoint)
            .arg(dmg)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("hdiutil を起動できません: {}", e))?;
        if let Some(mut stdin) = child.stdin.take() {
            // Apple の DMG は使用許諾への同意を求める。非対話なので stdin で Y を返す
            let _ = stdin.write_all(b"Y\nY\nY\nY\n");
        }
        let output = child.wait_with_output().map_err(|e| e.to_string())?;
        if !output.status.success() {
            let _ = std::fs::remove_dir(&mountpoint);
            return Err(format!(
                "DMG のマウントに失敗: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(Self { mountpoint })
    }

    pub fn path(&self) -> &Path {
        &self.mountpoint
    }
}

impl Drop for DmgMount {
    fn drop(&mut self) {
        let _ = std::process::Command::new("hdiutil")
            .args(["detach", "-quiet"])
            .arg(&self.mountpoint)
            .output();
        let _ = std::fs::remove_dir(&self.mountpoint);
    }
}

/// 取り込みの入口。`.dmg` / フォルダ / 単一 PNG のいずれかを受け付ける
pub fn import_frames(path: &Path, entries: &[DeviceEntry], user_dir: &Path) -> Result<ImportReport, String> {
    let is_dmg = path.extension().map_or(false, |x| x.eq_ignore_ascii_case("dmg"));
    if is_dmg {
        let mount = DmgMount::attach(path)?;
        let files = scan_pngs(mount.path());
        import_pngs(&files, mount.path(), false, entries, user_dir)
    } else if path.is_dir() {
        let files = scan_pngs(path);
        import_pngs(&files, path, true, entries, user_dir)
    } else if path.is_file() {
        let root = path.parent().unwrap_or(path);
        import_pngs(&[path.to_path_buf()], root, true, entries, user_dir)
    } else {
        Err(format!("取り込み元が見つかりません: {}", path.display()))
    }
}
```

- [ ] **Step 4: Tauri コマンドと opener を `main.rs` に追加**

`use frames::{catalog, compose, store};` を `use frames::{catalog, compose, import, store};` に変え、その直後に `use std::path::Path;` を追加。`list_frames` の直後に:

```rust
#[command]
fn import_frames(app: tauri::AppHandle, path: String) -> Result<import::ImportReport, String> {
    let roots = frame_roots(&app)?;
    let entries = load_catalog(&roots)?;
    import::import_frames(Path::new(&path), &entries, &roots.user)
}
```

`main()` を:

```rust
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_default_save_dir,
            select_element,
            capture_screenshots,
            abort_capture,
            list_frames,
            import_frames
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 5: 依存と権限**

`src-tauri/Cargo.toml` の `tauri-plugin-dialog = "2"` の次の行に:

```toml
tauri-plugin-opener = "2"
```

`src-tauri/capabilities/migrated.json` の `"permissions"` 配列末尾に:

```json
    "dialog:default",
    "opener:default"
```

npm 側:

Run: `pnpm add @tauri-apps/plugin-opener`
Expected: `package.json` の `dependencies` に `"@tauri-apps/plugin-opener": "^2.x"` が入る

- [ ] **Step 6: テストとビルド**

Run: `pnpm test 2>&1 | tail -5`
Expected: `test result: ok. 23 passed`（compose 6 + catalog 7 + store 4 + import 6）。`cargo build` で opener がリンクされる（初回は依存の取得で時間がかかる）

- [ ] **Step 7: Commit**

```bash
git add src-tauri/src/frames/import.rs src-tauri/src/main.rs src-tauri/Cargo.toml src-tauri/Cargo.lock src-tauri/capabilities/migrated.json package.json pnpm-lock.yaml
git commit -m "feat(frames): ✨ DMG マウント付き取り込みコマンドと opener プラグインを追加"
```

---

### Task 11: フロントエンド — 取り込み UI・公式リンク・色選択・ライセンス注記

**Files:**
- Modify: `src/components/DeviceFramePanel.vue`

**Interfaces:**
- Consumes: Task 10 の `import_frames(path)`、`@tauri-apps/plugin-opener` の `openUrl`、`@tauri-apps/plugin-dialog` の `open`、Task 8 の `ImportReport` 型

- [ ] **Step 1: script に取り込み・公式リンク・色選択を追加**

import 行を次に差し替え:

```ts
import { computed, onMounted, ref } from 'vue'
import { invoke } from '@tauri-apps/api/core'
import { open } from '@tauri-apps/plugin-dialog'
import { openUrl } from '@tauri-apps/plugin-opener'
import type { DeviceSelection, FrameStatus, ImportReport } from '../types/frames'
```

`defineExpose({ refresh })` の前に追加:

```ts
const importing = ref(false)

const openOfficial = async (url: string) => {
  try {
    await openUrl(url)
  } catch (e) {
    emit('status', `ブラウザを開けませんでした: ${e}`)
  }
}

const runImport = async (path: string) => {
  importing.value = true
  emit('status', 'フレームを取り込んでいます...')
  try {
    const report = await invoke<ImportReport>('import_frames', { path })
    await refresh()
    const names = report.imported.map(i => `${i.id} (${i.variant})`).join(', ')
    const skipped = report.skipped.length
      ? ` / スキップ ${report.skipped.length} 件: ${report.skipped.map(s => s.reason).join('; ')}`
      : ''
    emit(
      'status',
      report.imported.length
        ? `取り込み完了: ${names}${skipped}`
        : `取り込めるフレームがありませんでした${skipped}`
    )
  } catch (e) {
    emit('status', `取り込みエラー: ${e}`)
  } finally {
    importing.value = false
  }
}

const importDmg = async () => {
  const picked = await open({
    multiple: false,
    directory: false,
    filters: [{ name: 'Apple Product Bezels', extensions: ['dmg', 'png'] }],
  })
  if (typeof picked === 'string') await runImport(picked)
}

const importFolder = async () => {
  const picked = await open({ multiple: false, directory: true })
  if (typeof picked === 'string') await runImport(picked)
}

const variantOf = (id: string) => selected.value.find(s => s.id === id)?.variant ?? null

const setVariant = (id: string, variant: string) => {
  selected.value = selected.value.map(s => (s.id === id ? { id, variant } : s))
}
```

- [ ] **Step 2: テンプレートに色セレクタ・公式リンク・取り込みボタン・ライセンス注記を追加**

各デバイスの `<label>` 内、状態バッジ `</span>` の直後に:

```vue
          <select
            v-if="f.state === 'imported' && f.variants.length > 1 && isSelected(f.id)"
            :value="variantOf(f.id)"
            @change="setVariant(f.id, ($event.target as HTMLSelectElement).value)"
            class="text-xs bg-gray-100 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded px-1 py-0.5"
          >
            <option v-for="v in f.variants" :key="v" :value="v">{{ v }}</option>
          </select>
          <button
            v-if="f.state === 'missing' && f.source_url"
            type="button"
            @click.prevent="openOfficial(f.source_url ?? '')"
            class="text-xs text-blue-500 hover:text-blue-600 dark:text-blue-400 underline"
          >
            公式
          </button>
```

Apple グループの `<div class="flex flex-wrap gap-3">…</div>` の直後（`v-for="g in groups"` の div 内）に:

```vue
      <div v-if="g.vendor === 'apple'" class="mt-3 flex flex-wrap items-center gap-2">
        <span class="text-xs text-gray-500 dark:text-gray-400">
          公式サイトの Product Bezels から DMG をダウンロードして取り込んでください。
        </span>
        <button
          type="button"
          @click="importDmg"
          :disabled="importing"
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50"
        >
          DMG / PNG を取り込む
        </button>
        <button
          type="button"
          @click="importFolder"
          :disabled="importing"
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600 disabled:opacity-50"
        >
          フォルダを取り込む
        </button>
      </div>
```

`</section>` の直前（最後の `v-for` div の後）に:

```vue
    <p class="mt-4 text-xs text-gray-500 dark:text-gray-400">
      Apple のベゼルは Apple のライセンスに従いご自身の責任で使用してください。影の追加は Apple のガイドライン上は改変に当たります。Pixel のフレームは AOSP 由来（Apache 2.0）です。
    </p>
```

- [ ] **Step 3: 型チェック**

Run: `pnpm run build 2>&1 | tail -5`
Expected: エラーなし

- [ ] **Step 4: 手動 E2E（GUI。人が行う）— Apple 取り込みから撮影まで**

1. `pnpm run tauri:dev`。Apple 行の「公式」で https://developer.apple.com/design/resources/ が既定ブラウザで開く
2. iPhone 16 の DMG（`Bezel-iPhone-16.dmg`、約 270MB）をダウンロードし「DMG / PNG を取り込む」で選択 → ステータスに「取り込み完了: apple-iphone-16 (black), … apple-iphone-16-pro-max (white-titanium)」（4 機種 × 4〜5 色 = 18 件）。`~/Library/Application Support/com.responsiveshot.app/frames/apple-iphone-16-pro/` に 4 つの PNG。`hdiutil info | grep responsiveshot-dmg` が空（detach 済み）
3. Apple 4 件のバッジが「取り込み済み」になり、iPhone 16 Pro をチェックすると色セレクタが出る。`black-titanium` を選び、シャドウ ON、`https://example.com` で実行
4. `~/Downloads/capture_apple-iphone-16-pro_black-titanium_framed-shadow.png` が 1554×2964（1350+2·102 × 2760+2·102）で、画面がベゼルに収まり Dynamic Island 部分はページ内容が透けて見える（v1 仕様）
5. シャドウ OFF → `capture_apple-iphone-16-pro_black-titanium_framed.png` が 1350×2760
6. 「フォルダを取り込む」で DMG から手動コピーした `PNG/iPhone 16` フォルダを選んでも取り込める
7. 取り込み済み PNG を Finder で削除してから撮影 → 「フレームが見つかりません: iPhone 16 Pro (black-titanium)。取り込みをやり直してください」

- [ ] **Step 5: Commit**

```bash
git add src/components/DeviceFramePanel.vue
git commit -m "feat(ui): ✨ Apple ベゼルの取り込み・公式リンク・色選択・ライセンス注記を追加"
```

---

### Task 12: ドキュメント（README 日英・AGENTS.md）と最終確認

**Files:**
- Modify: `README.md`（Features に 1 行、新節 "Device frames / デバイスフレーム"）
- Modify: `AGENTS.md`（主要機能に 4 項目目、制約に「7.」節）

- [ ] **Step 1: README の Features に追記**

既存の Features リスト（GIF recording の行付近）に:

```markdown
- Device frames: capture at a real device's resolution and composite into official Apple / Google Pixel bezels, with optional drop shadow (PNG only)
```

- [ ] **Step 2: README に新節を追加（Usage の後）**

```markdown
## Device frames / デバイスフレーム

**English**

Select devices in the "デバイスフレーム" panel to capture the page at that device's CSS size and pixel ratio and save it composited into the device bezel (`capture_<device>_framed.png`, or `…_framed-shadow.png` with the drop-shadow toggle). Frames are applied to PNG output only.

- **Google Pixel** (Pixel 9 / 9 Pro / 9 Pro XL / 9a / 10 / 10 Pro / 10 Pro XL / 10a / Pixel Tablet) frames are bundled. They are derived from the Android Open Source Project (Apache License 2.0, see `src-tauri/frames/google/NOTICE`). Regenerate with `scripts/build-pixel-frames.sh` (requires ImageMagick).
- **Apple iPhone 16 / 16 Plus / 16 Pro / 16 Pro Max** bezels are not bundled because Apple's license does not allow redistribution. Download the "Product Bezels" DMG from [Apple Design Resources](https://developer.apple.com/design/resources/#product-bezels) yourself and use "DMG / PNG を取り込む" in the panel; the PNGs are copied to `~/Library/Application Support/com.responsiveshot.app/frames/`. Use them under Apple's [marketing guidelines](https://developer.apple.com/app-store/marketing/guidelines/) at your own responsibility (adding a shadow counts as a modification under those guidelines).

**日本語**

「デバイスフレーム」パネルで端末を選ぶと、その端末の CSS 寸法・ピクセル比で撮影し、ベゼルにはめ込んだ PNG（`capture_<device>_framed.png`、ドロップシャドウ ON なら `…_framed-shadow.png`）を保存します。フレームは PNG 出力のみに適用されます。

- **Google Pixel**（Pixel 9 / 9 Pro / 9 Pro XL / 9a / 10 / 10 Pro / 10 Pro XL / 10a / Pixel Tablet）のフレームは同梱しています。Android Open Source Project 由来（Apache License 2.0、`src-tauri/frames/google/NOTICE` 参照）。`scripts/build-pixel-frames.sh` で再生成できます（ImageMagick が必要）。
- **Apple iPhone 16 / 16 Plus / 16 Pro / 16 Pro Max** のベゼルは Apple のライセンス上再配布できないため同梱していません。[Apple Design Resources](https://developer.apple.com/design/resources/#product-bezels) から「Product Bezels」の DMG をご自身でダウンロードし、パネルの「DMG / PNG を取り込む」で取り込んでください。PNG は `~/Library/Application Support/com.responsiveshot.app/frames/` にコピーされます。Apple の[マーケティングガイドライン](https://developer.apple.com/app-store/marketing/guidelines/)に従いご自身の責任で使用してください（影の追加はガイドライン上の改変に当たります）。
```

- [ ] **Step 3: AGENTS.md を更新**

「主要機能」の 3. の後に:

```markdown
4. **デバイスフレーム (Device Frames)**
   - カタログ `src-tauri/frames/catalog.json` に登録した端末（Apple iPhone 16 系 4 機種・Google Pixel 9/10 系 + Pixel Tablet）を選ぶと、その CSS 寸法・DPR・mobile で viewport 撮影し、Rust 側 `frames::compose` でベゼル PNG に合成して保存する。ドロップシャドウはアプリが生成する
   - Google 分は同梱（AOSP, Apache 2.0）、Apple 分はユーザーが公式 DMG を取り込む（`frames::import`、`hdiutil attach` を使用）
```

「既知の制約」の 6. の後に:

```markdown
### 7. デバイスフレーム合成の設計制約
- **Apple のベゼル画像はリポジトリにもアプリにも含めない。** App Store Marketing Artwork License が再配布を認めていないため。カタログにはメタデータ（画面矩形・公式 DL URL・ファイル名パターン）だけを持ち、画像はユーザーが取り込む
- **カタログの不変条件**（`frames::catalog::validate` とテストで検証）: `id` は一意で英小文字・数字・ハイフンのみ、`screen` は `frame` に内包、`import` の `pattern` は `{variant}` をちょうど 1 回含む、`bundled` の PNG は存在して `frame` 寸法と一致
- **合成器は常に画面矩形へ cover リサイズする。** Pixel 9 の DPR 2.625 のような端数（412×2.625 = 1081.5）や Retina での返却倍率のブレを吸収するため。角丸クリップはしない（フレーム側の角が不透明でスクショの角を覆う）
- **シャドウのパラメータは固定**: `sigma = 0.015 × 幅`、`offset_y = 0.015 × 高さ`、不透明度 0.35、パディング `3σ + offset_y`。ぼかしは 1/4 縮小で行う（フルサイズだと 1470×3000 で数秒かかる）
- **幅指定の出力は変えない。** `CaptureTarget` の幅ターゲットは `dpr 1.0 / mobile false` 固定で、ファイル名も従来どおり
- **デバイスターゲットは viewport / PNG 固定。** GIF と同時指定は Rust 側で `Err` にする（フロントは GIF 選択時に `devices: []` を送る）
- Apple の Product Bezels PNG は Dynamic Island 部分も透明なので、ページ内容が透けて見える（v1 仕様。黒塗りは将来拡張）
```

- [ ] **Step 4: 最終確認**

Run: `pnpm test 2>&1 | tail -5 && git status --short`
Expected: `test result: ok. 23 passed`、未コミットの変更が README.md と AGENTS.md だけ

- [ ] **Step 5: Commit**

```bash
git add README.md AGENTS.md
git commit -m "docs: 📖 デバイスフレーム機能の使い方（日英）と設計制約を追記"
```

- [ ] **Step 6: PR 前の手順（人が行う）**

ユーザーのワークフローに従い、PR 作成前に `@codex-rescue` をフォアグラウンドで回してから `feature/device-frames` → `develop` の PR を作る。PR 本文は日英併記。
