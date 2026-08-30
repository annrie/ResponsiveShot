# デバイスフレーム v1.1 §3 Dynamic Island の黒塗り 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** iPhone 16 / 16 Plus / 16 Pro / 16 Pro Max のフレーム付き出力で、Dynamic Island の位置を黒い角丸矩形で塗り、実機の見た目に近づける。

**Architecture:** カタログに任意フィールド `island`（CSS px）を持たせ、`build_targets` が `FrameJob` に運び、`compose_png` がスクショ（cover リサイズ前の生画像）に `compose::fill_rounded_rect` で黒を描いてから従来の `compose_frame` に渡す。合成器の署名は増やさない。トグルは設けない（データがあれば常時適用）。

**Tech Stack:** Rust（`image` 0.24、serde）。フロントの変更なし。

**Spec:** `docs/superpowers/specs/2026-08-30-device-frame-v1.1-design.md` §3（§4 は別計画）

> **追記（2026-08-31、`2811f61`）**: 最終レビューで Apple のベゼル PNG が Dynamic Island を**不透明に描いている**ことが判明し、本計画の「公開 pt 値を採用」（Global Constraints・Task 1 の表・Task 4 の文言）は **ベゼル PNG からの実測値**（16 = 134/11/125×37.33/r18.67、16 Plus = 152.33/11.33/125.67×36.67/r18.33、16 Pro = 138.67/14.33/124.67×36/r18、16 Pro Max = 157.67/14.33/124.67×36/r18）に差し替えた。最終状態は spec §3「採用値」と `catalog.json` を正とする。

## Global Constraints

- `island` は **iPhone 16 / 16 Plus / 16 Pro / 16 Pro Max の 4 件だけ**に付ける。他機種は省略（`None`）で出力は従来と完全に同じ
- 値は **公開されている pt 値**（幅 126pt、高さ 37.33pt、上端 11pt、水平中央、radius = 高さ ÷ 2 = 18.67）。この Mac の iOS シミュレータは 15 分経っても起動しなかったため実測は断念し、spec §3 にその旨を記す（Task 4）
- 描画はスクショ（撮影直後の `css.width × dpr` px 画像）に対して行い、倍率は **`shot.width / css.width`**（dpr の丸めに依存しない）。`compose_frame` の引数は増やさない
- 黒は `Rgba([0, 0, 0, 255])`。境界は距離関数で 1px アンチエイリアス
- 新規依存なし、`let ... else` なし、`#[allow(dead_code)]` なし。`pnpm test` 合格、`cargo check` 警告 0。ユーザー向けエラー文言は英語、コードコメントは日本語
- コミットメッセージは `<type>(scope): <emoji> 日本語`。`.superpowers/`、`dist/`、`dist-release/` はコミットしない
- 各 Task の `cargo test` は `cd src-tauri && cargo test 2>&1 | tail -3` で確認（現在 44 件。Task 1 で +3、Task 2 で +3、Task 3 で +2 → 52 件）

## File Structure

| パス | 責務 |
|---|---|
| `src-tauri/src/frames/catalog.rs` | `Island` 型、`DeviceEntry.island: Option<Island>`、`validate` の island 検証 |
| `src-tauri/frames/catalog.json` | iPhone 4 件に `island` を追加 |
| `src-tauri/src/frames/compose.rs` | `fill_rounded_rect`（純関数、テスト付き） |
| `src-tauri/src/frames/targets.rs` | `FrameJob.island` / `FrameJob.css_width`、`build_targets` での受け渡し |
| `src-tauri/src/main.rs` | `compose_png` で island を描く |
| `docs/superpowers/specs/2026-08-30-device-frame-v1.1-design.md`, `README.md`, `AGENTS.md` | 公開値採用の注記と説明 |

---

### Task 1: カタログの `island` フィールドと検証

**Files:**
- Modify: `src-tauri/src/frames/catalog.rs`（`Island` 型、`DeviceEntry` のフィールド、`validate`、テスト）
- Modify: `src-tauri/frames/catalog.json`（iPhone 4 件）
- Modify: `DeviceEntry { ... }` のリテラルを持つ全テスト（`grep -rn "DeviceEntry {" src-tauri/src` で洗い出し、`island: None,` を追加）

**Interfaces:**
- Consumes: `super::Rect`、`CssSpec`（既存）
- Produces: `pub struct Island { pub x: f64, pub y: f64, pub width: f64, pub height: f64, pub radius: f64 }`（`Debug, Clone, Copy, PartialEq, Serialize, Deserialize`）、`DeviceEntry.island: Option<Island>`（JSON キー `island`、省略可）

- [ ] **Step 1: 失敗するテストを書く**

`catalog.rs` の `mod tests` に追加（既存の `sample()` 相当のヘルパがあればそれを使い、無ければ `parse_catalog` に JSON 文字列を渡す）:

```rust
    #[test]
    fn island_is_optional_and_deserializes() {
        let json = r#"[{
          "id": "apple-iphone-16", "vendor": "apple", "category": "phone", "name": "iPhone 16",
          "orientation": "portrait",
          "css": { "width": 393, "height": 852, "dpr": 3, "mobile": true },
          "frame": { "width": 1359, "height": 2736 },
          "screen": { "x": 90, "y": 90, "width": 1179, "height": 2556 },
          "source": { "kind": "import", "url": "https://example.com/x.dmg", "pattern": "PNG/{variant}.png" },
          "island": { "x": 133.5, "y": 11, "width": 126, "height": 37.33, "radius": 18.67 }
        }, {
          "id": "google-pixel-9", "vendor": "google", "category": "phone", "name": "Pixel 9",
          "orientation": "portrait",
          "css": { "width": 412, "height": 923, "dpr": 2.625, "mobile": true },
          "frame": { "width": 1200, "height": 2500 },
          "screen": { "x": 60, "y": 60, "width": 1080, "height": 2400 },
          "source": { "kind": "bundled", "file": "google/pixel-9.png" }
        }]"#;
        let entries = parse_catalog(json).unwrap();
        let island = entries[0].island.expect("iPhone 16 has an island");
        assert_eq!(island, Island { x: 133.5, y: 11.0, width: 126.0, height: 37.33, radius: 18.67 });
        assert_eq!(entries[1].island, None, "island を省略した機種は None");
    }

    #[test]
    fn rejects_island_outside_css_viewport() {
        let mut e = parse_catalog(SAMPLE).unwrap(); // e[1] は iPhone 16 Pro（css.width 402）
        e[1].island = Some(Island { x: 300.0, y: 11.0, width: 126.0, height: 37.33, radius: 18.67 }); // 300 + 126 > 402
        assert!(validate(&e).unwrap_err().contains("island must fit inside the css viewport"));
        e[1].island = Some(Island { x: 138.0, y: 11.0, width: 126.0, height: 37.33, radius: 30.0 }); // radius > height / 2
        assert!(validate(&e).unwrap_err().contains("island must fit inside the css viewport"));
        e[1].island = Some(Island { x: 138.0, y: 11.0, width: 126.0, height: 37.33, radius: 18.67 });
        assert!(validate(&e).is_ok(), "収まっていれば合格");
    }

    #[test]
    fn bundled_catalog_has_islands_only_on_iphone_16_family() {
        let entries = load_catalog(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/frames/catalog.json"))).unwrap();
        let with: Vec<&str> = entries.iter().filter(|e| e.island.is_some()).map(|e| e.id.as_str()).collect();
        assert_eq!(with, ["apple-iphone-16", "apple-iphone-16-plus", "apple-iphone-16-pro", "apple-iphone-16-pro-max"]);
        for e in entries.iter().filter(|e| e.island.is_some()) {
            let i = e.island.unwrap();
            assert!((i.x + i.width / 2.0 - e.css.width as f64 / 2.0).abs() < 0.01, "{}: island is centered", e.id);
            assert_eq!((i.y, i.width, i.height, i.radius), (11.0, 126.0, 37.33, 18.67), "{}", e.id);
        }
    }
```

- [ ] **Step 2: 失敗を確認**

Run: `cd src-tauri && cargo test frames::catalog 2>&1 | tail -5`
Expected: コンパイルエラー（`Island` / `island` が未定義）

- [ ] **Step 3: 実装**

`catalog.rs` の `CssSpec` の直後に追加:

```rust
/// Dynamic Island の黒塗り領域（CSS px、画面左上原点）。radius は角丸半径（高さ ÷ 2 でピル形）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Island {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub radius: f64,
}
```

`DeviceEntry` に（`source` の後ろ）:

```rust
    /// iPhone 16 系だけが持つ。無い機種は黒塗りしない
    #[serde(default)]
    pub island: Option<Island>,
```

`validate` の `screen rect exceeds the frame` チェックの直後に:

```rust
        if let Some(i) = e.island {
            let fits = i.width > 0.0
                && i.height > 0.0
                && i.x >= 0.0
                && i.y >= 0.0
                && i.x + i.width <= e.css.width as f64
                && i.y + i.height <= e.css.height as f64
                && i.radius >= 0.0
                && i.radius <= i.height / 2.0 + 1e-6;
            if !fits {
                return Err(format!("{}: island must fit inside the css viewport", e.id));
            }
        }
```

`catalog.json` の 4 件に `island` を追加（`source` の後ろ、各行はファイルの既存インデントに合わせる）:

| id | island |
|---|---|
| `apple-iphone-16` | `{ "x": 133.5, "y": 11, "width": 126, "height": 37.33, "radius": 18.67 }` |
| `apple-iphone-16-plus` | `{ "x": 152, "y": 11, "width": 126, "height": 37.33, "radius": 18.67 }` |
| `apple-iphone-16-pro` | `{ "x": 138, "y": 11, "width": 126, "height": 37.33, "radius": 18.67 }` |
| `apple-iphone-16-pro-max` | `{ "x": 157, "y": 11, "width": 126, "height": 37.33, "radius": 18.67 }` |

（x = (css.width − 126) / 2。393 → 133.5、430 → 152、402 → 138、440 → 157）

`DeviceEntry { ... }` のリテラルは `src-tauri/src/frames/import.rs` のテストヘルパ `entry()`（17 行目付近）だけにある。そこに `island: None,` を追加してコンパイルを通す（`catalog.rs` / `targets.rs` のテストは `parse_catalog(SAMPLE)` で生成しているので `#[serde(default)]` により変更不要）。

- [ ] **Step 4: 合格を確認**

Run: `cd src-tauri && cargo test 2>&1 | tail -3 && cargo check 2>&1 | grep -c warning`
Expected: `47 passed`、警告 `0`

- [ ] **Step 5: コミット**

```bash
git add src-tauri/src/frames/catalog.rs src-tauri/frames/catalog.json src-tauri/src/frames/targets.rs src-tauri/src/frames/store.rs src-tauri/src/frames/import.rs
git commit -m "feat(frames): ✨ カタログに Dynamic Island の領域（island）を追加し iPhone 16 系に設定"
```

（`git add` は実際に変更したファイルだけにする）

---

### Task 2: `compose::fill_rounded_rect`

**Files:**
- Modify: `src-tauri/src/frames/compose.rs`（関数 + テスト 3 件）

**Interfaces:**
- Produces: `pub fn fill_rounded_rect(img: &mut RgbaImage, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Rgba<u8>)`。画像外にはみ出す矩形は切り詰め、`w <= 0 || h <= 0` は何もしない。`radius` は `min(w, h) / 2` に丸める

- [ ] **Step 1: 失敗するテストを書く**

`compose.rs` の `mod tests` に追加（`solid` ヘルパは既存）:

```rust
    #[test]
    fn fill_rounded_rect_paints_inside_and_keeps_rounded_corners() {
        let mut img = solid(100, 60, [200, 0, 0, 255]);
        fill_rounded_rect(&mut img, 10.0, 10.0, 60.0, 30.0, 15.0, Rgba([0, 0, 0, 255]));
        assert_eq!(img.get_pixel(40, 25).0, [0, 0, 0, 255], "内側は黒");
        assert_eq!(img.get_pixel(15, 25).0, [0, 0, 0, 255], "左端中央（半円の内側）も黒");
        assert_eq!(img.get_pixel(10, 10).0, [200, 0, 0, 255], "外接矩形の角は角丸の外なので元の色");
        assert_eq!(img.get_pixel(5, 5).0, [200, 0, 0, 255], "矩形の外は元の色");
        assert_eq!(img.get_pixel(40, 45).0, [200, 0, 0, 255], "矩形の下も元の色");
    }

    #[test]
    fn fill_rounded_rect_antialiases_fractional_edges() {
        let mut img = solid(100, 60, [200, 0, 0, 255]);
        // 上辺が y = 10.5 なので、行 10（画素中心 10.5）は半分だけ覆われる
        fill_rounded_rect(&mut img, 10.0, 10.5, 60.0, 30.0, 0.0, Rgba([0, 0, 0, 255]));
        let edge = img.get_pixel(40, 10).0;
        assert!((95..=105).contains(&edge[0]) && edge[1] == 0 && edge[2] == 0, "境界は中間値 (got {:?})", edge);
        assert_eq!(img.get_pixel(40, 9).0, [200, 0, 0, 255], "その 1 行上は元の色");
        assert_eq!(img.get_pixel(40, 11).0, [0, 0, 0, 255], "その 1 行下は黒");
    }

    #[test]
    fn fill_rounded_rect_clips_to_image_and_ignores_empty_rect() {
        let mut img = solid(100, 60, [200, 0, 0, 255]);
        fill_rounded_rect(&mut img, 90.0, 50.0, 40.0, 40.0, 5.0, Rgba([0, 0, 0, 255])); // 画像外にはみ出しても落ちない
        assert_eq!(img.get_pixel(95, 55).0, [0, 0, 0, 255]);
        let before = img.clone();
        fill_rounded_rect(&mut img, 10.0, 10.0, 0.0, 30.0, 5.0, Rgba([0, 0, 0, 255]));
        assert_eq!(img, before, "幅 0 は何もしない");
    }
```

- [ ] **Step 2: 失敗を確認**

Run: `cd src-tauri && cargo test frames::compose 2>&1 | tail -5`
Expected: コンパイルエラー（`fill_rounded_rect` が未定義）

- [ ] **Step 3: 実装**

`compose.rs` の `cover_resize` の直後に追加:

```rust
/// `img` に角丸矩形を `color` で塗る（Dynamic Island の黒塗り用）。
/// 画素中心から角丸矩形への符号付き距離で被覆率を求め、境界を 1px で補間する。
/// 画像外にはみ出す部分は切り詰め、`w <= 0` または `h <= 0` なら何もしない。
pub fn fill_rounded_rect(img: &mut RgbaImage, x: f32, y: f32, w: f32, h: f32, radius: f32, color: Rgba<u8>) {
    if w <= 0.0 || h <= 0.0 {
        return;
    }
    let r = radius.max(0.0).min(w / 2.0).min(h / 2.0);
    let (cx, cy) = (x + w / 2.0, y + h / 2.0);
    // 角丸を除いた「芯」の半サイズ
    let (hx, hy) = (w / 2.0 - r, h / 2.0 - r);
    let x0 = (x.floor() - 1.0).max(0.0) as u32;
    let y0 = (y.floor() - 1.0).max(0.0) as u32;
    let x1 = ((x + w).ceil() + 1.0).max(0.0).min(img.width() as f32) as u32;
    let y1 = ((y + h).ceil() + 1.0).max(0.0).min(img.height() as f32) as u32;
    for py in y0..y1 {
        for px in x0..x1 {
            let dx = (px as f32 + 0.5 - cx).abs() - hx;
            let dy = (py as f32 + 0.5 - cy).abs() - hy;
            // 角丸矩形の符号付き距離（外側が正）
            let dist = (dx.max(0.0).powi(2) + dy.max(0.0).powi(2)).sqrt() + dx.max(dy).min(0.0) - r;
            let cover = (0.5 - dist).clamp(0.0, 1.0);
            if cover <= 0.0 {
                continue;
            }
            let p = img.get_pixel_mut(px, py);
            for c in 0..3 {
                p[c] = (color[c] as f32 * cover + p[c] as f32 * (1.0 - cover)).round() as u8;
            }
            p[3] = p[3].max((color[3] as f32 * cover).round() as u8);
        }
    }
}
```

- [ ] **Step 4: 合格を確認**

Run: `cd src-tauri && cargo test 2>&1 | tail -3 && cargo check 2>&1 | grep -c warning`
Expected: `50 passed`、警告 `0`

- [ ] **Step 5: コミット**

```bash
git add src-tauri/src/frames/compose.rs
git commit -m "feat(frames): ✨ 角丸矩形を塗る compose::fill_rounded_rect を追加"
```

---

### Task 3: `FrameJob` に island を運び `compose_png` で描く

**Files:**
- Modify: `src-tauri/src/frames/targets.rs`（`FrameJob` に 2 フィールド、`build_targets`、テスト 2 件）
- Modify: `src-tauri/src/main.rs:410-420`（`compose_png`）

**Interfaces:**
- Consumes: `catalog::Island`（Task 1）、`compose::fill_rounded_rect`（Task 2）
- Produces: `FrameJob { frame_png, screen, shadow, background, island: Option<Island>, css_width: u32 }`

- [ ] **Step 1: 失敗するテストを書く**

`targets.rs` の `mod tests` に追加。既存の `bundled_device_target_uses_catalog_css_and_screen` と同じ準備（`catalog::parse_catalog(SAMPLE)` + `roots(tag)` + `touch(...)`）を使い、同梱 Pixel 9 のエントリに island を手で付けて検証する（import 機種はフレーム PNG の解決が要るので使わない）:

```rust
    #[test]
    fn frame_job_carries_island_and_css_width() {
        let mut entries = catalog::parse_catalog(SAMPLE).unwrap();
        let island = catalog::Island { x: 143.0, y: 11.0, width: 126.0, height: 37.33, radius: 18.67 };
        entries[0].island = Some(island); // google-pixel-9（css.width 412）に付ける
        let r = roots("island");
        touch(&r.bundled.join("google/pixel_9.png"), 1198, 2531);
        let devices = [DeviceSelection { id: "google-pixel-9".into(), variant: None }];
        let targets = build_targets(&[], None, 1080, &devices, false, None, 0, Some((&entries, &r))).unwrap();
        let job = targets[0].frame.as_ref().unwrap();
        assert_eq!(job.css_width, 412);
        assert_eq!(job.island, Some(island));
    }

    #[test]
    fn frame_job_has_no_island_when_catalog_omits_it() {
        let entries = catalog::parse_catalog(SAMPLE).unwrap();
        let r = roots("no-island");
        touch(&r.bundled.join("google/pixel_9.png"), 1198, 2531);
        let devices = [DeviceSelection { id: "google-pixel-9".into(), variant: None }];
        let targets = build_targets(&[], None, 1080, &devices, false, None, 0, Some((&entries, &r))).unwrap();
        assert_eq!(targets[0].frame.as_ref().unwrap().island, None);
    }
```

（`build_targets` の引数順は既存テストと同じ: `widths, viewport_height, capture_height, devices, frame_shadow, frame_background, duration, frames`）

- [ ] **Step 2: 失敗を確認**

Run: `cd src-tauri && cargo test frames::targets 2>&1 | tail -5`
Expected: コンパイルエラー（`css_width` / `island` が `FrameJob` に無い）

- [ ] **Step 3: 実装**

`targets.rs` の `FrameJob`:

```rust
pub struct FrameJob {
    pub frame_png: PathBuf,
    pub screen: Rect,
    pub shadow: bool,
    /// フレーム外の背景色。None = 透明
    pub background: Option<Rgba<u8>>,
    /// Dynamic Island の黒塗り領域（CSS px）。None なら塗らない
    pub island: Option<Island>,
    /// island を実寸に換算するための CSS 幅（倍率 = shot.width / css_width）
    pub css_width: u32,
}
```

`use super::catalog::Island;`（既存の `use` の並びに合わせる。`targets.rs` が `catalog` を `use super::catalog;` で参照しているなら `catalog::Island` と書いてもよい）を追加し、`build_targets` の `FrameJob { ... }` 生成箇所（現在 89-94 行）に `island: entry.island, css_width: entry.css.width,` を追加。`FrameJob` の `#[derive(Debug)]` はそのまま（テストは `Option<Island>` 同士を比較するので `FrameJob: PartialEq` は不要）。

`main.rs` の `compose_png`（`use image::{imageops, ImageOutputFormat, RgbaImage};` に `Rgba` を加える）:

```rust
fn compose_png(shot_png: &[u8], job: &FrameJob, frame: &RgbaImage) -> Result<Vec<u8>, String> {
    let mut shot = image::load_from_memory(shot_png)
        .map_err(|e| format!("Failed to decode the screenshot: {}", e))?
        .to_rgba8();
    if let Some(i) = job.island {
        // 撮影画像は css.width × dpr px。dpr の丸めに依存しないよう実寸の幅から倍率を求める
        let s = shot.width() as f32 / job.css_width.max(1) as f32;
        compose::fill_rounded_rect(
            &mut shot,
            i.x as f32 * s,
            i.y as f32 * s,
            i.width as f32 * s,
            i.height as f32 * s,
            i.radius as f32 * s,
            Rgba([0, 0, 0, 255]),
        );
    }
    let out = compose::compose_frame(&shot, frame, job.screen, job.shadow, job.background);
    let mut buf = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(out)
        .write_to(&mut buf, ImageOutputFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;
    Ok(buf.into_inner())
}
```

`main.rs` に `use image::Rgba;` が無ければ追加（既存の `image` の `use` に合わせる）。

- [ ] **Step 4: 合格を確認**

Run: `cd src-tauri && cargo test 2>&1 | tail -3 && cargo check 2>&1 | grep -c warning && cd .. && pnpm run build 2>&1 | tail -1`
Expected: `52 passed`、警告 `0`、`✓ built`

- [ ] **Step 5: コミット**

```bash
git add src-tauri/src/frames/targets.rs src-tauri/src/main.rs
git commit -m "feat(frames): ✨ iPhone 16 系のフレーム付き出力で Dynamic Island を黒塗りする"
```

---

### Task 4: ドキュメント（spec の注記、README 日英、AGENTS.md）

**Files:**
- Modify: `docs/superpowers/specs/2026-08-30-device-frame-v1.1-design.md` §3
- Modify: `README.md`（`## Device frames / デバイスフレーム` 節）
- Modify: `AGENTS.md`（`### 7. デバイスフレーム合成の設計制約`）

- [ ] **Step 1: spec §3 に採用値の注記**

§3 の「値の求め方」の箇条書きの末尾に追加:

```markdown
- **採用値（2026-08-31）**: この Mac の iOS シミュレータ（x86 版 SimLaunchHost）は 15 分経っても起動が終わらなかったため実測は断念し、公開されている pt 値を採用した。iPhone 16 / 16 Plus / 16 Pro / 16 Pro Max 共通で幅 126pt・高さ 37.33pt・上端 11pt・水平中央（x = (css.width − 126) / 2）、radius = 18.67。実機と数 px ずれる可能性があるので、気になれば実測値で `catalog.json` の `island` を更新する
```

- [ ] **Step 2: README**

`## Device frames / デバイスフレーム` 節の英語側の箇条書きに 1 行、日本語側に 1 行（既存の箇条書きの体裁に合わせる）:

```markdown
- iPhone 16 / 16 Plus / 16 Pro / 16 Pro Max: the Dynamic Island area is painted black (published size 126 × 37.33 pt, 11 pt from the top; may differ from the device by a few px). No toggle.
```

```markdown
- iPhone 16 / 16 Plus / 16 Pro / 16 Pro Max は Dynamic Island の位置を黒で塗ります（公開されている寸法 126 × 37.33pt、上端 11pt。実機と数 px ずれる場合があります）。トグルはありません
```

- [ ] **Step 3: AGENTS.md §7 に追記**

`### 7.` の箇条書きの末尾に:

```markdown
* **Dynamic Island**: `catalog.json` の任意フィールド `island`（CSS px）。`build_targets` が `FrameJob.island` / `css_width` に運び、`compose_png` が **cover リサイズ前のスクショ**に `compose::fill_rounded_rect` で黒を描いてから `compose_frame` に渡す（倍率は `shot.width / css_width`）。合成器の署名は増やさない。`validate` が css ビューポート内に収まることを検証する
```

- [ ] **Step 4: 確認**

Run: `git diff --stat`
Expected: 3 ファイルのみ

- [ ] **Step 5: コミット**

```bash
git add docs/superpowers/specs/2026-08-30-device-frame-v1.1-design.md README.md AGENTS.md
git commit -m "docs: 📖 Dynamic Island 黒塗りの説明（日英）と公開値採用の注記"
```

---

## 完了後

1. ブランチ全体の最終レビュー → `@codex-rescue` → push → PR（`feature/frames-island` → develop、日英併記、「公開値採用・実機と数 px ずれる可能性」を明記）→ 本文なし `@codex review` → 指摘対応
2. 手動 E2E: iPhone 16 Pro Max + 白背景のページで上端中央に黒いピルが出ること / Pixel 9 の出力は従来どおり / 影・背景色との組み合わせで崩れないこと
3. マージ → main ff 同期 → ブランチ削除 → `.superpowers/sdd/2026-08-31-frames-island/` 削除
