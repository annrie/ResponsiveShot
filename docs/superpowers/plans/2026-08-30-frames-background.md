# デバイスフレーム v1.1 §2: 背景色オプション Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** フレーム付き出力の背景（透明部分）を 透明 / 白 / 黒 / 任意色 から選べるようにする。

**Architecture:** 合成器 `compose_frame` にキャンバス初期色（`Option<Rgba<u8>>`）を渡す 1 引数を追加し、`FrameJob` / `build_targets` / `capture_screenshots` を通して UI から `#rrggbb` を受け取る。hex の検証は撮影前（Rust）と入力時（Vue）の両方。幅指定キャプチャには影響しない。

**Tech Stack:** Rust（`image` 0.24）/ Vue 3 + TS

**Spec:** `docs/superpowers/specs/2026-08-30-device-frame-v1.1-design.md` §2

## Global Constraints

- 背景色は **フレーム付き出力のみ**。幅指定キャプチャの出力は変えない。ファイル名も変えない
- `frame_background: Option<String>`（`None` / 空文字 = 透明）。`#rgb` / `#rrggbb` のみ受け付け、不正値は **撮影前**に `Err("背景色の形式が不正です: …")`
- 影は背景色の上に描く（影の alpha はそのまま）。`None` のときは従来どおり完全に同じ出力
- 新規依存なし、`let ... else` なし、`#[allow(dead_code)]` なし。`pnpm test` 合格、`cargo check` 警告 0
- コミットメッセージは `<type>(scope): <emoji> 日本語`。`.superpowers/`、`dist/`、`dist-release/` はコミットしない

## File Structure

| パス | 責務 |
|---|---|
| `src-tauri/src/frames/compose.rs` | `parse_hex_color`、`compose_frame` の `background` 引数 |
| `src-tauri/src/frames/targets.rs` | `FrameJob.background`、`build_targets` の `frame_background` 引数 |
| `src-tauri/src/main.rs` | `capture_screenshots` の `frame_background` 引数、hex 検証、`compose_png` への受け渡し |
| `src/components/DeviceFramePanel.vue` | 背景セレクト + 任意色入力（`v-model:background`） |
| `src/App.vue` | `rs-frame-bg` の永続化、invoke 引数、入力検証 |
| `README.md`, `AGENTS.md` | 説明 |

---

### Task 1: 合成器 — hex パースとキャンバス初期色

**Files:**
- Modify: `src-tauri/src/frames/compose.rs`

**Interfaces:**
- Produces: `pub fn parse_hex_color(s: &str) -> Result<Rgba<u8>, String>`、`pub fn compose_frame(shot, frame, screen, shadow: bool, background: Option<Rgba<u8>>) -> RgbaImage`

- [ ] **Step 1: テストを追加し、既存 4 か所の `compose_frame(...)` 呼び出しに第 5 引数 `None` を足す**

`tests` モジュール末尾に追加:

```rust
    #[test]
    fn parse_hex_color_accepts_3_and_6_digits() {
        assert_eq!(parse_hex_color("#fff").unwrap(), Rgba([255, 255, 255, 255]));
        assert_eq!(parse_hex_color("#1a2B3c").unwrap(), Rgba([26, 43, 60, 255]));
        assert_eq!(parse_hex_color("  #000 ").unwrap(), Rgba([0, 0, 0, 255]), "前後の空白は無視");
    }

    #[test]
    fn parse_hex_color_rejects_invalid() {
        for s in ["fff", "#ggg", "#12345", "", "#", "#1234567", "white"] {
            let err = parse_hex_color(s).unwrap_err();
            assert!(err.contains("背景色の形式が不正です"), "{s}: {err}");
        }
    }

    #[test]
    fn background_fills_transparent_areas_and_shadow_darkens_it() {
        let shot = solid(60, 140, [200, 0, 0, 255]);
        let mut frame = frame_with_hole();
        frame.put_pixel(0, 0, Rgba([0, 0, 0, 0])); // フレーム外周に透明画素を 1 つ

        // 背景なし: 従来どおり透明
        let out = compose_frame(&shot, &frame, HOLE, false, None);
        assert_eq!(out.get_pixel(0, 0)[3], 0);

        // 白背景・影なし: 透明画素が白に、画面とベゼルは不変
        let white = Rgba([255, 255, 255, 255]);
        let out = compose_frame(&shot, &frame, HOLE, false, Some(white));
        assert_eq!(out.get_pixel(0, 0).0, [255, 255, 255, 255]);
        assert_eq!(out.get_pixel(50, 100).0, [200, 0, 0, 255]);
        assert_eq!(out.get_pixel(5, 5).0, BEZEL);

        // 白背景・影あり: 四隅は純白、本体下は白が暗くなる（黒い影が乗る）
        let p = ShadowParams::for_frame(100, 200);
        let out = compose_frame(&shot, &frame, HOLE, true, Some(white));
        assert_eq!(out.get_pixel(0, 0).0, [255, 255, 255, 255]);
        let below = out.get_pixel(p.padding + 50, p.padding + 200 + 1);
        assert_eq!(below[3], 255, "背景ありなら不透明");
        assert!(below[0] < 250 && below[0] == below[1] && below[1] == below[2], "白の上に黒の影 → 灰色: {:?}", below);
    }
```

- [ ] **Step 2: 失敗を確認**

Run: `cd src-tauri && cargo test frames::compose 2>&1 | tail -15`
Expected: `parse_hex_color` 未定義と引数個数のコンパイルエラー

- [ ] **Step 3: 実装**

`cover_resize` の前に追加:

```rust
/// `#rgb` / `#rrggbb` を不透明色に変換する。前後の空白は無視。それ以外は Err
pub fn parse_hex_color(s: &str) -> Result<Rgba<u8>, String> {
    let err = || format!("背景色の形式が不正です: {:?}（#rrggbb 形式で指定してください）", s);
    let t = s.trim();
    let hex = t.strip_prefix('#').ok_or_else(err)?;
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(err());
    }
    let expanded: String = match hex.len() {
        3 => hex.chars().flat_map(|c| [c, c]).collect(),
        6 => hex.to_string(),
        _ => return Err(err()),
    };
    let channel = |i: usize| u8::from_str_radix(&expanded[i..i + 2], 16).map_err(|_| err());
    Ok(Rgba([channel(0)?, channel(2)?, channel(4)?, 255]))
}
```

`compose_frame` を次に置き換え（変更点は署名と `canvas` の初期化のみ）:

```rust
/// スクショを `screen` に cover リサイズして置き、その上にフレームを重ねる。
/// `shadow` が true ならキャンバスを `padding` 分広げ、影 → スクショ → フレーム の順に重ねる。
/// `background` を指定するとキャンバスをその色（不透明）で初期化する。`None` は透明（従来どおり）。
/// フレームの画面部分は透明である前提（Apple / Google の公式素材はどちらもそう）。
/// 角丸クリップはしない: フレーム側の角が不透明でスクショの角を覆う。
pub fn compose_frame(
    shot: &RgbaImage,
    frame: &RgbaImage,
    screen: Rect,
    shadow: bool,
    background: Option<Rgba<u8>>,
) -> RgbaImage {
    let fitted = cover_resize(shot, screen.width, screen.height);
    let params = ShadowParams::for_frame(frame.width(), frame.height());
    let pad = if shadow { params.padding } else { 0 };

    let (cw, ch) = (frame.width() + 2 * pad, frame.height() + 2 * pad);
    let mut canvas = match background {
        Some(color) => RgbaImage::from_pixel(cw, ch, color),
        None => RgbaImage::new(cw, ch),
    };
    if shadow {
        imageops::overlay(&mut canvas, &shadow_layer(frame, screen, &params), 0, 0);
    }
    imageops::overlay(&mut canvas, &fitted, (pad + screen.x) as i64, (pad + screen.y) as i64);
    imageops::overlay(&mut canvas, frame, pad as i64, pad as i64);
    canvas
}
```

- [ ] **Step 4: テスト**

Run: `cd src-tauri && cargo test frames::compose 2>&1 | tail -4`
Expected: `test result: ok. 9 passed`（既存 6 + 3）。この時点で `main.rs` の呼び出しが引数不足でコンパイルエラーになる場合は、`main.rs` の `compose::compose_frame(&shot, frame, job.screen, job.shadow)` を暫定で `..., job.shadow, None)` にして通す（Task 2 で置き換える）

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/frames/compose.rs src-tauri/src/main.rs
git commit -m "feat(frames): ✨ 合成器に背景色（hex）を指定できるようにする"
```

---

### Task 2: `FrameJob` / `build_targets` / `capture_screenshots` に背景色を通す

**Files:**
- Modify: `src-tauri/src/frames/targets.rs`
- Modify: `src-tauri/src/main.rs`

**Interfaces:**
- Consumes: Task 1 の `parse_hex_color` / `compose_frame(.., background)`
- Produces: `FrameJob { frame_png, screen, shadow, background: Option<Rgba<u8>> }`、`build_targets(widths, viewport_height, capture_height, devices, frame_shadow, frame_background: Option<Rgba<u8>>, duration, frames)`、Tauri 引数 `frame_background: Option<String>`（JS からは `frameBackground: string | null`）

- [ ] **Step 1: targets.rs のテストを更新・追加**

既存 7 か所の `build_targets(...)` 呼び出しで、`frame_shadow` の引数（`false` / `true`）の直後に `None` を挿入する（例: `build_targets(&widths, None, 1080, &[], false, None, 0, None)`）。`bundled_device_target_uses_catalog_css_and_screen` の直後に追加:

```rust
    #[test]
    fn device_target_carries_background() {
        let entries = parse_catalog(SAMPLE).unwrap();
        let r = roots("bg");
        touch(&r.bundled.join("google/pixel_9.png"), 1198, 2531);
        let devices = [DeviceSelection { id: "google-pixel-9".into(), variant: None }];
        let white = image::Rgba([255, 255, 255, 255]);
        let targets = build_targets(&[], None, 1080, &devices, false, Some(white), 0, Some((&entries, &r))).unwrap();
        assert_eq!(targets[0].frame.as_ref().unwrap().background, Some(white));
        let targets = build_targets(&[], None, 1080, &devices, false, None, 0, Some((&entries, &r))).unwrap();
        assert_eq!(targets[0].frame.as_ref().unwrap().background, None);
    }
```

（`touch` ヘルパーの引数が現在 `(path, w, h)` であることを確認して合わせる）

- [ ] **Step 2: 失敗を確認**

Run: `cd src-tauri && cargo test frames::targets 2>&1 | tail -8`
Expected: 引数個数 / フィールド未定義のコンパイルエラー

- [ ] **Step 3: targets.rs の実装**

```rust
use image::Rgba;   // 既存 use の並びに追加

pub struct FrameJob {
    pub frame_png: PathBuf,
    pub screen: Rect,
    pub shadow: bool,
    /// フレーム外の背景色。None = 透明
    pub background: Option<Rgba<u8>>,
}
```

`build_targets` の署名に `frame_background: Option<Rgba<u8>>,` を `frame_shadow: bool,` の直後に追加し、`FrameJob { … shadow: frame_shadow, background: frame_background }` とする。

- [ ] **Step 4: main.rs の実装**

`capture_screenshots` の引数 `frame_shadow: bool,` の直後に `frame_background: Option<String>,` を追加。`let frames_ctx = …` の前に:

```rust
    // 背景色は撮影前に検証する（不正値でブラウザを起動しない）
    let frame_background = match frame_background.as_deref().map(str::trim) {
        Some("") | None => None,
        Some(s) => Some(compose::parse_hex_color(s)?),
    };
```

`build_targets(` の呼び出しに `frame_shadow,` の直後で `frame_background,` を渡す。`compose_png` 内を `compose::compose_frame(&shot, frame, job.screen, job.shadow, job.background)` にする（Task 1 で暫定 `None` にした場合はここで置き換え）。

- [ ] **Step 5: テスト**

Run: `cd src-tauri && cargo test 2>&1 | tail -3 && cargo check 2>&1 | grep -c warning`
Expected: `38 passed`（34 + Task 1 の 3 + 1）、warning `0`

- [ ] **Step 6: Commit**

```bash
git add src-tauri/src/frames/targets.rs src-tauri/src/main.rs
git commit -m "feat(frames): ✨ 背景色を capture_screenshots から合成器まで受け渡す"
```

---

### Task 3: UI（背景セレクト + 任意色）とドキュメント

**Files:**
- Modify: `src/components/DeviceFramePanel.vue`
- Modify: `src/App.vue`
- Modify: `README.md`, `AGENTS.md`

**Interfaces:**
- Consumes: Task 2 の `frameBackground: string | null`
- Produces: `DeviceFramePanel` の `v-model:background`（`'transparent' | '#rrggbb'`）

- [ ] **Step 1: DeviceFramePanel.vue の script**

`const shadow = defineModel<boolean>('shadow', { required: true })` の直後に:

```ts
/** 'transparent' か '#rrggbb'（'#rgb' も可）。App.vue 側で rs-frame-bg に永続化 */
const background = defineModel<string>('background', { required: true })

const HEX_RE = /^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/
const isValidHex = (s: string) => HEX_RE.test(s.trim())

type BgMode = 'transparent' | 'white' | 'black' | 'custom'
const bgMode = computed<BgMode>(() => {
  const v = background.value.trim().toLowerCase()
  if (v === 'transparent') return 'transparent'
  if (v === '#ffffff' || v === '#fff') return 'white'
  if (v === '#000000' || v === '#000') return 'black'
  return 'custom'
})
const setBgMode = (mode: BgMode) => {
  if (mode === 'transparent') background.value = 'transparent'
  else if (mode === 'white') background.value = '#ffffff'
  else if (mode === 'black') background.value = '#000000'
  else if (bgMode.value !== 'custom') background.value = '#f5f5f5'
}
const backgroundInvalid = computed(() => bgMode.value === 'custom' && !isValidHex(background.value))
```

- [ ] **Step 2: DeviceFramePanel.vue の template**

ヘッダー行のシャドウ `<label>` を、次のコントロール群で置き換える（`<h2>` はそのまま）:

```vue
      <div class="flex items-center gap-4 text-sm">
        <label class="flex items-center gap-1">
          <span class="text-xs text-gray-500 dark:text-gray-400">背景</span>
          <select
            :value="bgMode"
            @change="setBgMode(($event.target as HTMLSelectElement).value as BgMode)"
            class="text-xs bg-gray-100 dark:bg-gray-900 border border-gray-200 dark:border-gray-700 rounded px-1 py-0.5"
          >
            <option value="transparent">透明</option>
            <option value="white">白</option>
            <option value="black">黒</option>
            <option value="custom">任意</option>
          </select>
          <input
            v-if="bgMode === 'custom'"
            :value="background"
            @input="background = ($event.target as HTMLInputElement).value"
            type="text"
            placeholder="#rrggbb"
            spellcheck="false"
            class="w-24 text-xs font-mono bg-gray-100 dark:bg-gray-900 border rounded px-1 py-0.5"
            :class="backgroundInvalid ? 'border-red-400' : 'border-gray-200 dark:border-gray-700'"
          />
        </label>
        <label class="flex items-center gap-2 cursor-pointer">
          <input type="checkbox" v-model="shadow" class="text-blue-500 rounded" />
          ドロップシャドウ
        </label>
      </div>
```

- [ ] **Step 3: App.vue**

`const frameShadow = useStorage('rs-frame-shadow', false)` の直後に:

```ts
const frameBackground = useStorage('rs-frame-bg', 'transparent')
```

`captureScreenshots` の `devices` を求めた直後（幅/デバイスのバリデーションの後）に:

```ts
  if (devices.length > 0 && frameBackground.value !== 'transparent' && !/^#([0-9a-fA-F]{3}|[0-9a-fA-F]{6})$/.test(frameBackground.value.trim())) {
    statusMessage.value = "背景色は #rrggbb 形式で指定してください。"
    return
  }
```

`invoke` の引数 `frameShadow: frameShadow.value` の直後に `frameBackground: frameBackground.value === 'transparent' ? null : frameBackground.value.trim()` を追加。テンプレートの `<DeviceFramePanel …>` に `v-model:background="frameBackground"` を追加。

- [ ] **Step 4: ビルド**

Run: `pnpm run build 2>&1 | tail -3`
Expected: エラーなし

- [ ] **Step 5: ドキュメント**

README の Device frames 節、EN の Drop shadow の記述がある箇条書きの直後に:

```markdown
- **Background**: framed output is transparent by default; choose white, black or any `#rrggbb` in the panel to bake an opaque background (useful for viewers like Preview.app that render transparency as black).
```

JA 側に:

```markdown
- **背景**: フレーム付き出力は既定で透明です。パネルで 白 / 黒 / 任意の `#rrggbb` を選ぶと不透明な背景が焼き込まれます（透明を黒く表示するプレビュー.app などで確認する場合に便利）。
```

（該当の箇条書きが無ければ Apple の箇条書きの直後に追加。）AGENTS.md §7 末尾に:

```markdown
- 背景色（`frame_background`）はフレーム付き出力のみに適用し、`compose_frame` のキャンバス初期色として実装している。hex の検証は撮影前（`parse_hex_color`）と UI の両方で行う
```

- [ ] **Step 6: テストとコミット**

Run: `pnpm test 2>&1 | tail -3`
Expected: `38 passed`

```bash
git add src/components/DeviceFramePanel.vue src/App.vue README.md AGENTS.md
git commit -m "feat(ui): ✨ フレーム付き出力の背景色（透明 / 白 / 黒 / 任意）を選べるようにする"
```

- [ ] **Step 7: 手動 E2E（人が行う）**

1. `pnpm run tauri:dev` → 「背景」セレクトが「透明」で、従来どおり透明 PNG が出る
2. 「白」で Pixel 9・影あり → プレビュー.app でも白背景の上に影が見える。四隅が純白
3. 「任意」→ `#f0f4ff` → 出力の隅がその色。`#zz` など不正値は入力欄が赤くなり、実行時に「背景色は #rrggbb 形式で指定してください。」
4. 幅 375 のみ（デバイスなし）の出力は従来どおり
