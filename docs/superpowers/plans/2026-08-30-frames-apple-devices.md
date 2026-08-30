# デバイスフレーム v1.1 §1: Apple 機種追加 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** iPad Pro / iPad Air / iPad mini（縦・横）、MacBook Air / Pro、iMac、Studio Display のフレームをカタログに追加し、パネルをカテゴリ別に整理する。

**Architecture:** 追加はデータ（`catalog.json` に 17 エントリ）と UI（カテゴリ小見出し、公式リンク、説明文）のみ。取り込み・合成・撮影の Rust コードは変更しない（既存の `pattern` 照合で新 DMG の構成を扱える）。

**Tech Stack:** JSON データ / Vue 3 + TS / Rust テスト（件数のみ）

**Spec:** `docs/superpowers/specs/2026-08-30-device-frame-v1.1-design.md` §1（実測値の表はそこが正）

## Global Constraints

- Rust の合成・取り込み・撮影ロジックは変更しない。変更するのは `src-tauri/frames/catalog.json`、`src-tauri/src/frames/catalog.rs` のテスト件数、`src/types/frames.ts`、`src/components/DeviceFramePanel.vue`、`README.md`、`AGENTS.md` のみ
- カタログ不変条件（id 一意・`[a-z0-9-]`、screen ⊂ frame、`{variant}` ちょうど 1 回）を満たす。新エントリはすべて `source.kind: "import"`
- iMac の screen は和集合（x 140, y 150, w **4482**, h 2520）
- パターン中の `"` は JSON で `\"` にエスケープする
- 新規依存なし。`pnpm test`（vue-tsc + vite build + cargo test）が通ること、`cargo check` 警告 0
- コミットメッセージは `<type>(scope): <emoji> 日本語`。`.superpowers/`、`dist/`、`dist-release/` はコミットしない

## File Structure

| パス | 責務 |
|---|---|
| `src-tauri/frames/catalog.json` | 17 エントリ追加（Apple の後・Google の前に並べる） |
| `src-tauri/src/frames/catalog.rs` | `bundled_catalog_is_valid_and_bundled_pngs_match_frame_size` の件数 13 → 30 |
| `src/types/frames.ts` | `category` union の拡張 |
| `src/components/DeviceFramePanel.vue` | カテゴリ小見出し、Design Resources 固定リンク、3 ステップ文の一般化 |
| `README.md`, `AGENTS.md` | 対応機種の一覧と設計メモ |

---

### Task 1: カタログに Apple 17 機種を追加

**Files:**
- Modify: `src-tauri/frames/catalog.json`
- Modify: `src-tauri/src/frames/catalog.rs`（テスト 1 か所）

**Interfaces:**
- Produces: `list_frames` が 30 件を返す（新 17 件は `state: "missing"`、`source_url` は各 DMG の直リンク）

- [ ] **Step 1: テストの件数を 30 にして失敗を確認**

`src-tauri/src/frames/catalog.rs` の `assert_eq!(entries.len(), 13);` を `assert_eq!(entries.len(), 30);` に変更。

Run: `cd src-tauri && cargo test bundled_catalog 2>&1 | tail -5`
Expected: FAIL（`left: 13, right: 30`）

- [ ] **Step 2: `catalog.json` の Apple 4 件の直後（`google-pixel-9` の前）に次の 17 件を挿入**

```json
  {
    "id": "apple-ipad-pro-11-m5-portrait",
    "vendor": "apple", "category": "tablet", "name": "iPad Pro 11\" (M5) 縦", "orientation": "portrait",
    "css": { "width": 834, "height": 1210, "dpr": 2.0, "mobile": true },
    "frame": { "width": 1880, "height": 2640 },
    "screen": { "x": 106, "y": 110, "width": 1668, "height": 2420 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-iPad-Pro-(M5).dmg",
      "pattern": "PNG/iPad Pro (M5) 11\" - {variant} - Portrait.png" }
  },
  {
    "id": "apple-ipad-pro-11-m5-landscape",
    "vendor": "apple", "category": "tablet", "name": "iPad Pro 11\" (M5) 横", "orientation": "landscape",
    "css": { "width": 1210, "height": 834, "dpr": 2.0, "mobile": true },
    "frame": { "width": 2640, "height": 1880 },
    "screen": { "x": 110, "y": 106, "width": 2420, "height": 1668 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-iPad-Pro-(M5).dmg",
      "pattern": "PNG/iPad Pro (M5) 11\" - {variant} - Landscape.png" }
  },
  {
    "id": "apple-ipad-pro-13-m5-portrait",
    "vendor": "apple", "category": "tablet", "name": "iPad Pro 13\" (M5) 縦", "orientation": "portrait",
    "css": { "width": 1032, "height": 1376, "dpr": 2.0, "mobile": true },
    "frame": { "width": 2300, "height": 3000 },
    "screen": { "x": 118, "y": 124, "width": 2064, "height": 2752 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-iPad-Pro-(M5).dmg",
      "pattern": "PNG/iPad Pro (M5) 13\" - {variant} - Portrait.png" }
  },
  {
    "id": "apple-ipad-pro-13-m5-landscape",
    "vendor": "apple", "category": "tablet", "name": "iPad Pro 13\" (M5) 横", "orientation": "landscape",
    "css": { "width": 1376, "height": 1032, "dpr": 2.0, "mobile": true },
    "frame": { "width": 3000, "height": 2300 },
    "screen": { "x": 124, "y": 118, "width": 2752, "height": 2064 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-iPad-Pro-(M5).dmg",
      "pattern": "PNG/iPad Pro (M5) 13\" - {variant} - Landscape.png" }
  },
  {
    "id": "apple-ipad-air-11-m4-portrait",
    "vendor": "apple", "category": "tablet", "name": "iPad Air 11\" (M4) 縦", "orientation": "portrait",
    "css": { "width": 820, "height": 1180, "dpr": 2.0, "mobile": true },
    "frame": { "width": 1900, "height": 2620 },
    "screen": { "x": 130, "y": 130, "width": 1640, "height": 2360 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-iPad-Air-(M4).dmg",
      "pattern": "PNG/iPad Air 11\" (M4) - {variant} - Portrait.png" }
  },
  {
    "id": "apple-ipad-air-11-m4-landscape",
    "vendor": "apple", "category": "tablet", "name": "iPad Air 11\" (M4) 横", "orientation": "landscape",
    "css": { "width": 1180, "height": 820, "dpr": 2.0, "mobile": true },
    "frame": { "width": 2620, "height": 1900 },
    "screen": { "x": 130, "y": 130, "width": 2360, "height": 1640 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-iPad-Air-(M4).dmg",
      "pattern": "PNG/iPad Air 11\" (M4) - {variant} - Landscape.png" }
  },
  {
    "id": "apple-ipad-air-13-m4-portrait",
    "vendor": "apple", "category": "tablet", "name": "iPad Air 13\" (M4) 縦", "orientation": "portrait",
    "css": { "width": 1024, "height": 1366, "dpr": 2.0, "mobile": true },
    "frame": { "width": 2300, "height": 2980 },
    "screen": { "x": 126, "y": 124, "width": 2048, "height": 2732 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-iPad-Air-(M4).dmg",
      "pattern": "PNG/iPad Air 13\" (M4) - {variant} - Portrait.png" }
  },
  {
    "id": "apple-ipad-air-13-m4-landscape",
    "vendor": "apple", "category": "tablet", "name": "iPad Air 13\" (M4) 横", "orientation": "landscape",
    "css": { "width": 1366, "height": 1024, "dpr": 2.0, "mobile": true },
    "frame": { "width": 2980, "height": 2300 },
    "screen": { "x": 124, "y": 126, "width": 2732, "height": 2048 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-iPad-Air-(M4).dmg",
      "pattern": "PNG/iPad Air 13\" (M4) - {variant} - Landscape.png" }
  },
  {
    "id": "apple-ipad-mini-a17-pro-portrait",
    "vendor": "apple", "category": "tablet", "name": "iPad mini (A17 Pro) 縦", "orientation": "portrait",
    "css": { "width": 744, "height": 1133, "dpr": 2.0, "mobile": true },
    "frame": { "width": 1780, "height": 2550 },
    "screen": { "x": 146, "y": 142, "width": 1488, "height": 2266 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-iPad-mini-(A17-Pro).dmg",
      "pattern": "PNG/iPad mini (A17 Pro) - {variant} - Portrait.png" }
  },
  {
    "id": "apple-ipad-mini-a17-pro-landscape",
    "vendor": "apple", "category": "tablet", "name": "iPad mini (A17 Pro) 横", "orientation": "landscape",
    "css": { "width": 1133, "height": 744, "dpr": 2.0, "mobile": true },
    "frame": { "width": 2550, "height": 1780 },
    "screen": { "x": 142, "y": 146, "width": 2266, "height": 1488 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-iPad-mini-(A17-Pro).dmg",
      "pattern": "PNG/iPad mini (A17 Pro) - {variant} - Landscape.png" }
  },
  {
    "id": "apple-macbook-air-13-m5",
    "vendor": "apple", "category": "laptop", "name": "MacBook Air 13\" (M5)", "orientation": "landscape",
    "css": { "width": 1280, "height": 832, "dpr": 2.0, "mobile": false },
    "frame": { "width": 3400, "height": 2240 },
    "screen": { "x": 420, "y": 288, "width": 2560, "height": 1664 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-MacBook-Air-M5.dmg",
      "pattern": "PNG/MacBook Air M5 13-inch {variant}.png" }
  },
  {
    "id": "apple-macbook-air-15-m5",
    "vendor": "apple", "category": "laptop", "name": "MacBook Air 15\" (M5)", "orientation": "landscape",
    "css": { "width": 1440, "height": 932, "dpr": 2.0, "mobile": false },
    "frame": { "width": 3540, "height": 2300 },
    "screen": { "x": 329, "y": 218, "width": 2880, "height": 1864 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-MacBook-Air-M5.dmg",
      "pattern": "PNG/MacBook Air M5 15-inch {variant}.png" }
  },
  {
    "id": "apple-macbook-pro-14-m5",
    "vendor": "apple", "category": "laptop", "name": "MacBook Pro 14\" (M5)", "orientation": "landscape",
    "css": { "width": 1512, "height": 982, "dpr": 2.0, "mobile": false },
    "frame": { "width": 3860, "height": 2540 },
    "screen": { "x": 418, "y": 288, "width": 3024, "height": 1964 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-MacBook-Pro-M5.dmg",
      "pattern": "PNG/MacBook Pro M5 14-inch {variant}.png" }
  },
  {
    "id": "apple-macbook-pro-16-m5",
    "vendor": "apple", "category": "laptop", "name": "MacBook Pro 16\" (M5)", "orientation": "landscape",
    "css": { "width": 1728, "height": 1117, "dpr": 2.0, "mobile": false },
    "frame": { "width": 4260, "height": 2840 },
    "screen": { "x": 402, "y": 303, "width": 3456, "height": 2234 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-MacBook-Pro-M5.dmg",
      "pattern": "PNG/MacBook Pro M5 16-inch {variant}.png" }
  },
  {
    "id": "apple-imac-24-m4",
    "vendor": "apple", "category": "desktop", "name": "iMac 24\" (M4)", "orientation": "landscape",
    "css": { "width": 2240, "height": 1260, "dpr": 2.0, "mobile": false },
    "frame": { "width": 4760, "height": 4050 },
    "screen": { "x": 140, "y": 150, "width": 4482, "height": 2520 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-iMac-M4.dmg",
      "pattern": "PNG/iMac M4 24-inch {variant}.png" }
  },
  {
    "id": "apple-studio-display-2026",
    "vendor": "apple", "category": "display", "name": "Studio Display (2026)", "orientation": "landscape",
    "css": { "width": 2560, "height": 1440, "dpr": 2.0, "mobile": false },
    "frame": { "width": 5400, "height": 4160 },
    "screen": { "x": 140, "y": 140, "width": 5120, "height": 2880 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-Studio-Displays.dmg",
      "pattern": "PNG/Studio Display 2026 {variant}.png" }
  },
  {
    "id": "apple-studio-display-xdr-2026",
    "vendor": "apple", "category": "display", "name": "Studio Display XDR (2026)", "orientation": "landscape",
    "css": { "width": 2560, "height": 1440, "dpr": 2.0, "mobile": false },
    "frame": { "width": 5400, "height": 4160 },
    "screen": { "x": 140, "y": 140, "width": 5120, "height": 2880 },
    "source": { "kind": "import",
      "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-Studio-Displays.dmg",
      "pattern": "PNG/Studio Display XDR 2026 {variant}.png" }
  },
```

- [ ] **Step 3: JSON の妥当性と件数を確認してからテスト**

Run: `python3 -c "import json;d=json.load(open('src-tauri/frames/catalog.json'));print(len(d));print(sum(1 for e in d if e['vendor']=='apple'))"`
Expected: `30` と `21`

Run: `cd src-tauri && cargo test frames::catalog 2>&1 | tail -4`
Expected: `test result: ok. 7 passed`

- [ ] **Step 4: Commit**

```bash
git add src-tauri/frames/catalog.json src-tauri/src/frames/catalog.rs
git commit -m "feat(frames): ✨ iPad / MacBook / iMac / Studio Display の 17 機種をカタログに追加"
```

---

### Task 2: パネルのカテゴリ小見出し・公式リンク・説明文

**Files:**
- Modify: `src/types/frames.ts:5`
- Modify: `src/components/DeviceFramePanel.vue`（`groups` computed、`officialUrl`、テンプレートのグループ描画、3 ステップ文）

**Interfaces:**
- Consumes: `FrameStatus.category` に `laptop` / `desktop` / `display` が来る（Task 1）

- [ ] **Step 1: 型を拡張**

`src/types/frames.ts` の `category: 'phone' | 'tablet'` を:

```ts
  category: 'phone' | 'tablet' | 'laptop' | 'desktop' | 'display'
```

- [ ] **Step 2: script — ラベルとグループ構造**

`const vendorLabels …` の直後に追加:

```ts
const categoryOrder: FrameStatus['category'][] = ['phone', 'tablet', 'laptop', 'desktop', 'display']
const categoryLabels: Record<FrameStatus['category'], string> = {
  phone: 'スマートフォン',
  tablet: 'タブレット',
  laptop: 'ノート PC',
  desktop: 'デスクトップ',
  display: 'ディスプレイ',
}
const APPLE_DESIGN_RESOURCES = 'https://developer.apple.com/design/resources/#product-bezels'
```

`const groups = computed(...)` を次に置き換える（vendor → category の 2 段）:

```ts
const groups = computed(() =>
  (['apple', 'google'] as const)
    .map(vendor => ({
      vendor,
      label: vendorLabels[vendor],
      sections: categoryOrder
        .map(category => ({
          category,
          label: categoryLabels[category],
          items: frames.value.filter(f => f.vendor === vendor && f.category === category),
        }))
        .filter(s => s.items.length > 0),
    }))
    .filter(g => g.sections.length > 0)
)
```

`const officialUrl = computed(...)` は削除する（未使用になるため）。

- [ ] **Step 3: template — グループ描画を 2 段にする**

`<div v-for="g in groups" :key="g.vendor" class="mb-4 last:mb-0">` の中の `<h3 …>{{ g.label }}</h3>` は残し、その直後の `<div class="flex flex-wrap gap-3">` … `</div>`（デバイスの `<label v-for="f in g.items">` を含むブロック）を次で包み直す。`<label>` の中身（checkbox / name / badge / select / 公式リンク）は変更しない:

```vue
      <div v-for="s in g.sections" :key="s.category" class="mb-3 last:mb-0">
        <h4 class="text-xs text-gray-500 dark:text-gray-400 mb-2">{{ s.label }}</h4>
        <div class="flex flex-wrap gap-3">
          <label
            v-for="f in s.items"
            :key="f.id"
            ...（既存の label 要素そのまま）...
          </label>
        </div>
      </div>
```

Apple グループの取り込みボタン列（`v-if="g.vendor === 'apple'"` の div）は sections の後に置いたまま。その中の 「公式サイトを開く」 ボタンを:

```vue
        <button
          type="button"
          @click="openOfficial(APPLE_DESIGN_RESOURCES)"
          class="px-3 py-1.5 text-xs font-medium rounded-md bg-gray-200 hover:bg-gray-300 dark:bg-gray-700 dark:hover:bg-gray-600"
        >
          公式サイトを開く
        </button>
```

（`:disabled="!officialUrl"` を外す）

- [ ] **Step 4: 3 ステップ文の一般化**

`② ブラウザで iPhone 16 の Product Bezels（DMG）をダウンロードしてください` を `② ブラウザで対応機種（iPhone / iPad / MacBook / iMac / Studio Display）の Product Bezels（DMG）をダウンロードしてください` に変更。

- [ ] **Step 5: 型チェックとビルド**

Run: `pnpm run build 2>&1 | tail -3`
Expected: エラーなし（`officialUrl` を消し忘れると未使用エラー、`g.items` を残すと型エラーになる）

- [ ] **Step 6: Commit**

```bash
git add src/types/frames.ts src/components/DeviceFramePanel.vue
git commit -m "feat(ui): ✨ デバイスフレームをカテゴリ別に表示し公式リンクを Design Resources に固定"
```

---

### Task 3: ドキュメントと手動 E2E チェックリスト

**Files:**
- Modify: `README.md`（Device frames 節の Apple 箇条書き、日英）
- Modify: `AGENTS.md`（§7 に 2 行）

- [ ] **Step 1: README の Apple 箇条書きを機種一覧付きに更新**

EN の `- **Apple iPhone 16 / 16 Plus / 16 Pro / 16 Pro Max** bezels are not bundled …` の機種部分を `**Apple iPhone 16 family, iPad Pro (M5) / iPad Air (M4) / iPad mini (A17 Pro) in portrait and landscape, MacBook Air / MacBook Pro (M5), iMac (M4), Studio Display (2026)**` に置き換え、末尾に `Each device lists which DMG to download; one DMG import covers every model, color and orientation it contains.` を追加。
JA の `- **Apple iPhone 16 / 16 Plus / 16 Pro / 16 Pro Max** のベゼルは…` を `**Apple iPhone 16 系、iPad Pro (M5) / iPad Air (M4) / iPad mini (A17 Pro) の縦・横、MacBook Air / MacBook Pro (M5)、iMac (M4)、Studio Display (2026)**` に置き換え、末尾に `各機種の行にダウンロードする DMG が示されます。DMG を 1 つ取り込むと、その中の全機種・色・向きがまとめて取り込まれます。` を追加。

- [ ] **Step 2: AGENTS.md §7 に追記**

§7 の末尾に:

```markdown
- iPad は縦・横を別エントリ（`-portrait` / `-landscape`）にしてある。Apple の DMG は iPhone 16 以外 `PNG/` 直下にファイルが並び、Mac 系は色の前の区切りが空白のみ。`pattern` の prefix/suffix 照合で吸収している
- iMac 24" (M4) の画面矩形は 7 色の穴の和集合（Orange だけ 2px 右にずれる）。余分な 2px はベゼルの下に隠れる
```

- [ ] **Step 3: 最終確認**

Run: `pnpm test 2>&1 | tail -3 && git status --short`
Expected: `31 passed`、未コミットは README.md / AGENTS.md のみ

- [ ] **Step 4: Commit**

```bash
git add README.md AGENTS.md
git commit -m "docs: 📖 Apple 追加機種の対応一覧と設計メモを追記"
```

- [ ] **Step 5: 手動 E2E（人が行う）**

1. `pnpm run tauri:dev` でパネルに Apple 21 件（スマートフォン 4 / タブレット 10 / ノート PC 4 / デスクトップ 1 / ディスプレイ 2）と Google 9 件がカテゴリ別に並ぶ
2. iPad Pro (M5) の DMG を取り込み → 4 エントリが「取り込み済み」。iPad Pro 13" 縦・横で撮影し、縦 2300×3000 / 横 3000×2300 の framed 出力
3. Studio Displays の DMG を取り込み → Studio Display (2026) で撮影。**5400×4160 の出力で画面（5120×2880）が欠けていない**ことを確認（欠ける場合は spec §1.3 のとおり設計変更）
4. iMac の DMG を取り込み、Orange で撮影 → 画面右端に透明の筋が出ない
