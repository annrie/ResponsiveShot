# デバイスフレーム合成機能 設計書

- 日付: 2026-08-30
- 対象: ResponsiveShot v1.0.1 (Tauri v2 / Vue 3 / Rust)
- 状態: レビュー待ち
- 参考: [c0bra/deviceframe](https://github.com/c0bra/deviceframe)（本機能の着想元。フレーム素材は Facebook Design "Devices" 由来で配布終了済み）

## 1. 目的

撮影したスクリーンショットを、実機のフレーム画像（ベゼル）にはめ込んだ PNG として出力できるようにする。ポートフォリオ等で「iPhone / Pixel / タブレットに表示された状態」を見せる用途。deviceframe が 7 年以上更新されておらず素材が古いため、**メーカー公式が配布しているフレーム素材だけ**を使って同等機能を ResponsiveShot に組み込む。

## 2. スコープ

### 2.1 v1 に含めるもの

- フレームカタログ（メタデータ JSON）と、フレーム画像の 2 系統の調達
  - Google Pixel: AOSP 由来（Apache 2.0）の画像を**アプリに同梱**
  - Apple: ユーザーが公式 DMG をダウンロードし、アプリの**取り込み機能**でコピー
- 「デバイスを選んで撮影」: 選択デバイスの CSS 寸法 / DPR / mobile エミュレーションで撮影し、フレームに合成して保存
- ドロップシャドウの ON/OFF（アプリ側で生成）
- 対象デバイス（すべて 1 向きのみ）
  - Apple: iPhone 16 / 16 Plus / 16 Pro / 16 Pro Max（縦、色は取り込めた分すべて）
  - Google: Pixel 9 / 9 Pro / 9 Pro XL / 9a / 10 / 10 Pro / 10 Pro XL / 10a（縦）、Pixel Tablet（横）
- Rust 側ユニットテスト（合成器・カタログ・ファイル名マッチ）
- README（日英）と AGENTS.md の更新

### 2.2 v1 に含めないもの（将来拡張）

- iPad / MacBook / iMac / Studio Display（DMG の形式が iPhone と同じことを実測してから追加。手順は §5.4）
- 横向き iPhone、縦向き Pixel Tablet、折りたたみ機（`closed` / `default` の 2 構成）、Watch、TV
- Dynamic Island の黒塗り（Apple のベゼル PNG は Island 部分も透明のため、ページ内容が透けて見える。v1 はそのまま）
- GIF へのフレーム合成
- 未知のフレーム画像の持ち込み（画面矩形の自動検出は §9 の方法で可能だが v1 では使わない）
- 素のスクリーンショットの同時保存（幅指定で別途撮影できる）
- Samsung / Dell / LG 等: 公式のフレーム素材が確認できないため対象外
- mobile UA / タッチのエミュレーション（現状は `mobile: true` のみ）
- Apple DMG 内のディレクトリ構成が変わった場合の耐性（現状は `PNG/<機種>/…` 固定、変更時はカタログ更新）
- 出力の背景色オプション（透明 / 白 / 任意色。Preview.app は透明を黒で描くため）
- 手動操作オーバーレイの狭い viewport（393px）での横幅調整

## 3. 確認済みの事実（設計の根拠）

2026-08-30 に実物を展開・実測した結果。

### 3.1 Apple — Design Resources "Product Bezels"

- 配布: https://developer.apple.com/design/resources/#product-bezels 。ログイン不要の DMG。EULA 付きで `hdiutil attach` 時に `Y` の応答が必要
- DMG 内構成（iPhone 16 の例）: `PNG/<機種>/<機種> - <色> - Portrait|Landscape.png` と `Photoshop/…psd`、`App Store Marketing Artwork License Agreement.rtf`
- PNG は RGBA。**画面領域は透明で、寸法は端末のネイティブ解像度と完全一致**。本体の外側も透明（余白 21〜25px、影は含まれない）。Dynamic Island 部分も透明
- 実測値:

| 機種 | フレーム PNG | 画面矩形 (x, y, w, h) | CSS (w × h @ DPR) | 色 |
|---|---|---|---|---|
| iPhone 16 | 1359×2736 | 90, 90, 1179, 2556 | 393×852 @3 | Black / Pink / Teal / Ultramarine / White |
| iPhone 16 Plus | 1470×2970 | 90, 87, 1290, 2796 | 430×932 @3 | 同上 |
| iPhone 16 Pro | 1350×2760 | 72, 69, 1206, 2622 | 402×874 @3 | Black / Desert / Natural / White Titanium |
| iPhone 16 Pro Max | 1470×3000 | 75, 66, 1320, 2868 | 440×956 @3 | 同上 |

- ライセンス: App Store Marketing Artwork License Agreement。App Store 公開アプリのマーケティング用途向けで、**第三者への再配布不可**、製品画像への「reflections, shadows, highlights の追加」等を改変として禁止。→ アプリには同梱せず、ユーザーが自分で取得したものを自分の環境で使う設計にする

### 3.2 Google — AOSP `tools/adt/idea` の `device-art-resources`

- 配布: https://android.googlesource.com/platform/tools/adt/idea/+/refs/heads/mirror-goog-studio-main/artwork/resources/device-art-resources/ 。各機種ディレクトリに `back.webp`（本体、画面部が透明）、`mask.webp`（画面と同寸の前景。カメラ穴と角の覆い）、`layout`（テキスト。display 寸法・オフセット・角丸半径）
- ライセンス: Apache License 2.0（`device-art.xml` ヘッダに明記）
- `back.webp` に影は含まれない（本体が画像端まで不透明）
- 実測値（`layout` から抽出）:

| id | display (w×h) | フレーム (w×h) | オフセット (x, y) | 角丸 | CSS (w × h @ DPR) |
|---|---|---|---|---|---|
| pixel_9 | 1080×2424 | 1198×2531 | 55, 58 | 87 | 412×923 @2.625 |
| pixel_9_pro | 1280×2856 | 1408×2974 | 60, 61 | 109 | 427×952 @3 |
| pixel_9_pro_xl | 1344×2992 | 1466×3101 | 57, 56 | 108 | 448×997 @3 |
| pixel_9a | 1080×2424 | 1224×2570 | 69, 73 | 87 | 412×923 @2.625 |
| pixel_10 | 1080×2424 | 1205×2535 | 59, 55 | 87 | 412×923 @2.625 |
| pixel_10_pro | 1280×2856 | 1410×2968 | 59, 60 | 99 | 427×952 @3 |
| pixel_10_pro_xl | 1344×2992 | 1472×3111 | 60, 55 | 108 | 448×997 @3 |
| pixel_10a | 1080×2424 | 1218×2553 | 65, 64 | 87 | 412×923 @2.625 |
| pixel_tablet | 2560×1600 | 2798×1837 | 119, 117 | — | 1280×800 @2（横） |

CSS 値は display ÷ DPR を四捨五入したもの。1〜2px の端数は合成器のリサイズで吸収する（§9）。

pixel_10 / 10_pro / 10_pro_xl は `layout` の記載値と `back.webp` の実寸が異なる（AOSP 側の不整合）ため、実寸を採用した（2026-08-30 実測）。

### 3.3 中央フラッドフィルによる画面矩形検出

フレーム PNG のアルファを 2 値化し、画像中央から透明ピクセルを塗りつぶした領域の外接矩形を取ると、Apple 4 機種と Pixel 9 のすべてで上表の画面矩形と一致した。v1 ではカタログに実測値を持ち、この検出は将来の「未知フレーム持ち込み」用に温存する。

## 4. アーキテクチャ概要

```
┌ Vue (App.vue) ─────────────────────────────────────────────┐
│ デバイスフレーム section                                     │
│  - list_frames() で状態表示  - import_frames(path)           │
│  - open_url(url) (opener)    - capture_screenshots(devices…) │
└──────────────┬─────────────────────────────────────────────┘
               │ invoke
┌ Rust (src-tauri/src) ──────────────────────────────────────┐
│ frames/catalog.rs   カタログ読込・型                          │
│ frames/store.rs     フレーム画像の解決（同梱 / 取り込み）      │
│ frames/import.rs    DMG/フォルダ走査・検証・コピー             │
│ frames/compose.rs   合成器（純関数）+ シャドウ                 │
│ main.rs             CaptureTarget 化したキャプチャループ       │
└────────────────────────────────────────────────────────────┘
同梱リソース: src-tauri/frames/catalog.json, frames/google/*.png, frames/google/LICENSE, NOTICE
ユーザー領域:  ~/Library/Application Support/com.responsiveshot.app/frames/<deviceId>/<variant>.png
```

`main.rs` は現在 792 行の単一ファイルなので、フレーム関連は `src-tauri/src/frames/` モジュールに分けて追加する。既存のキャプチャ関数群は移動しない。

## 5. データモデル — フレームカタログ

### 5.1 スキーマ

`src-tauri/frames/catalog.json`。配列で、1 デバイス 1 エントリ。

```jsonc
{
  "id": "apple-iphone-16-pro",          // 英小文字・数字・ハイフン。ファイル名にも使う
  "vendor": "apple",                    // "apple" | "google"
  "category": "phone",                  // "phone" | "tablet"
  "name": "iPhone 16 Pro",              // UI 表示名
  "orientation": "portrait",            // "portrait" | "landscape"（v1 は情報のみ）
  "css":    { "width": 402, "height": 874, "dpr": 3.0, "mobile": true },
  "frame":  { "width": 1350, "height": 2760 },
  "screen": { "x": 72, "y": 69, "width": 1206, "height": 2622 },
  "source": {
    "kind": "import",                   // "bundled" | "import"
    "url": "https://devimages-cdn.apple.com/design/resources/download/Bezel-iPhone-16.dmg",
    "pattern": "PNG/iPhone 16 Pro/iPhone 16 Pro - {variant} - Portrait.png"
  }
}
```

Google エントリの `source` は `{ "kind": "bundled", "file": "google/pixel_9.png" }`。`css.mobile` は phone / tablet とも `true`。

不変条件（テストで検証する）:

- `id` は一意
- `screen` は `frame` に内包される
- `bundled` エントリの `file` が同梱リソースに存在し、その PNG 寸法が `frame` と一致する
- `import` エントリの `pattern` は `{variant}` を 1 回だけ含む

### 5.2 v1 カタログの内容

§3.1 / §3.2 の表をそのまま転記する。Apple 4 件、Google 9 件。

### 5.3 同梱 Pixel 画像の生成

`scripts/build-pixel-frames.sh`（開発時のみ実行、ImageMagick 必須）:

1. AOSP から各機種の `back.webp` / `mask.webp` / `layout` を取得
2. `layout` から display 寸法とオフセットを読み、`mask.webp` を `back.webp` のオフセット位置に重ねた 1 枚の PNG を生成（`back` の画面部は透明なのでスクショの下に置くものは無く、実行時は「スクショの上にフレーム 1 枚」に統一できる）
3. `src-tauri/frames/google/<id>.png` に書き出す。**生成物はリポジトリにコミット**する（1 枚 ~100KB × 9）
4. `src-tauri/frames/google/LICENSE`（Apache 2.0 全文）と `NOTICE`（出典 URL・取得日）を置く

### 5.4 Apple 機種の追加手順（将来）

DMG を展開し、PNG の寸法と §3.3 のフラッドフィルで画面矩形を実測し、カタログにエントリを足す。コードの変更は不要。

## 6. フレーム画像の保存場所と解決

- 同梱: `tauri.conf.json` の `bundle.resources` に `frames/**/*` を追加し、`app.path().resource_dir()/frames/…` から読む
- 取り込み: `app.path().app_data_dir()/frames/<deviceId>/<variant-slug>.png`。`variant-slug` は色名を小文字化し空白を `-` にしたもの（`black-titanium`）。UI・出力ファイル名ともこの slug をそのまま使い、元の表記（`Black Titanium`）は保持しない
- 解決順: `source.kind == bundled` なら同梱のみ、`import` なら取り込み領域のみ（混在させない）

## 7. Rust コマンド API

```rust
#[command] fn list_frames(app: AppHandle) -> Result<Vec<FrameStatus>, String>
// FrameStatus { id, vendor, category, name, orientation,
//               state: "bundled" | "imported" | "missing",
//               variants: Vec<String>,        // 取り込み済みの色スラッグ（bundled は空）
//               source_url: Option<String> }

#[command] fn import_frames(app: AppHandle, path: String) -> Result<ImportReport, String>
// ImportReport { imported: Vec<{ id, variant }>, skipped: Vec<{ file, reason }> }

#[command] fn capture_screenshots(
    app: AppHandle,
    url, widths, mode, _format, selector, save_dir, duration, delay,
    manual_interaction, viewport_height,          // 既存
    devices: Vec<DeviceSelection>,                // 追加 { id: String, variant: Option<String> }
    frame_shadow: bool,                           // 追加
) -> Result<(), String>
```

公式ページを開くのは `tauri-plugin-opener` の `openUrl` をフロントから直接呼ぶ（Rust 側コマンドは不要）。

### 7.1 `import_frames` の動作

1. `path` が `.dmg` なら一時ディレクトリに `hdiutil attach -nobrowse -readonly -mountpoint <tmp> <dmg>` を実行し、stdin に `Y\n` を書いて EULA を承諾する。マウントは Drop ガードで必ず `hdiutil detach` する（エラー経路含む）
2. マウント先（またはフォルダ、または単一 PNG の親ディレクトリ）を再帰走査して `*.png` を集める（`._*` は除外）
3. カタログの `import` エントリごとに `pattern` を `{variant}` の位置で `prefix` / `suffix` に分割する。DMG の場合はボリューム相対パス全体が `prefix` で始まり `suffix` で終わるか、フォルダ/PNG の場合はファイル名が「`prefix` の最後のパス成分」で始まり `suffix` で終わるかで判定し、間の文字列を variant とする（正規表現 crate は使わない）
4. マッチしたファイルは PNG ヘッダから寸法を読み、`frame` と一致するものだけを保存領域にコピー。不一致は `skipped` に理由付きで記録
5. どのエントリにもマッチしなかったファイルは黙って無視する（DMG には PSD 等も入っている）

## 8. キャプチャ経路の変更（`main.rs`）

現在の `for w in widths` ループ本体（起動 → ナビゲート → 手動操作待ち → 撮影 → 保存 → 別スレッド drop）はそのまま残し、ループ対象を一般化する。

```rust
struct CaptureTarget {
    width: u32, height: u32,   // CSS px
    dpr: f64, mobile: bool,    // 幅指定: 1.0 / false。デバイス: css.dpr / css.mobile
    label: String,             // ファイル名用。幅指定は従来どおり "1440px" / "1440x810"
    frame: Option<FrameJob>,   // Some なら viewport 固定・PNG 固定・合成あり
}
struct FrameJob { device_id: String, variant: Option<String>, frame_png: PathBuf, screen: Rect, shadow: bool }
```

- `targets = widths → CaptureTarget` ＋ `devices → CaptureTarget`。デバイス分は `list_frames` と同じ解決でフレーム PNG のパスを確定し、無ければその時点でエラー（撮影を始めない）
- `set_viewport_metrics(tab, w, h)` を `(tab, &target)` に変え、`device_scale_factor` と `mobile` を渡す。幅ターゲットは従来と同じ値になるので**既存出力は変わらない**
- `LaunchOptionsBuilder::window_size` は CSS 寸法のまま
- デバイスターゲットは `mode` を無視して viewport 撮影。GIF はフロント側で送らない（§10 で section を無効化し `devices: []` を送る）。防御として Rust 側は `duration > 0 && !devices.is_empty()` なら撮影前に `Err("デバイスフレームは PNG 出力のみ対応しています")` を返す
- 撮影結果（`capture_screenshot` の PNG バイト列）を `image::load_from_memory` で読み、`compose_frame` に渡して `std::fs::write`
- 出力名: `capture_<deviceId>_<variant-slug>_framed.png`、影付きは `…_framed-shadow.png`。variant が無い（Google）場合は `capture_<deviceId>_framed.png`
- ABORT_FLAG チェック・別スレッド drop・手動操作 UI は既存のまま共有

## 9. 合成器（`frames/compose.rs`）

```rust
pub struct Rect { x: u32, y: u32, width: u32, height: u32 }
pub fn compose_frame(shot: &RgbaImage, frame: &RgbaImage, screen: Rect, shadow: bool, background: Option<Rgba<u8>>) -> RgbaImage
pub fn screen_mask(frame: &RgbaImage, screen: Rect) -> Vec<bool>
pub fn parse_hex_color(s: &str) -> Result<Rgba<u8>, String>
```

1. `shot` を `screen` に **cover** リサイズ（比率維持で覆い、中央クロップ。`FilterType::Lanczos3`）。寸法が既に一致していれば等倍
2. `screen_mask` でフレームの穴（画面中央から連結する非不透明画素）を求め、穴の外のスクショ画素を透明にする
3. `pad = shadow ? padding : 0`（§9.1）
4. `frame.width + 2·pad` × `frame.height + 2·pad` の`background` が Some ならその色で、None なら透明で初期化したキャンバス
5. shadow なら影レイヤー（キャンバスと同寸で、シルエットを `(pad, pad + offset_y)` にずらして描いたもの）を `(0, 0)` に overlay
6. リサイズ済み `shot` を `(pad + screen.x, pad + screen.y)` に overlay
7. `frame` を `(pad, pad)` に overlay

スクショはフレームの「穴」でクリップする（`screen_mask`：画面矩形の中央画素から非不透明画素をフラッドフィルして到達範囲を求め、到達しない画素は透明にする）。v1.1.0 リリース後の修正: 当初は「フレーム側の角が不透明でスクショの角を覆う」ためクリップ不要という前提だったが、Apple のベゼルは角の丸みが大きく画面矩形の角が本体の外（透明）に出るため誤りだった。

### 9.1 ドロップシャドウ

- パラメータはフレーム寸法比で固定。`sigma = 0.015 × frame.width`、`offset_y = 0.015 × frame.height`、`opacity = 0.35`、`padding = ceil(3·sigma + offset_y)`。iPhone 16 Pro なら sigma≈20px、offset≈41px、padding≈102px
- シルエット = `frame` のアルファ ∪ `screen` 矩形（画面部はフレームでは透明だが実機は塗り潰し）
- シルエットを 1/4 に縮小 → `imageops::blur(sigma/4)` → 元寸法に拡大 → 黒 × `opacity` の RGBA レイヤーにする
- 影の色は黒固定、背景は透明のまま（deviceframe と同じ）

### 9.2 メモリ

最大でフレーム 1466×3101 RGBA ≈ 18MB × 3 レイヤー。AGENTS.md の OOM 制約（GIF 200 コマ）とは桁が違うため追加対策は不要。

## 10. UI（`App.vue`）

「キャプチャする画面幅」section の直下に **「デバイスフレーム」section** を追加する。

- マウント時と取り込み後に `list_frames` を呼び、ベンダー別（Apple / Google）にチェックボックスで列挙
- 各行: デバイス名、状態バッジ（`同梱` / `取り込み済み` / `未取り込み`）、取り込み済みで色が複数ある場合は色セレクタ
- 未取り込みの Apple 行: 「公式サイトを開く」（`openUrl(source_url)`）と「DMG / フォルダを取り込む」（`open()` ダイアログ → `import_frames`）。取り込み結果は `ImportReport` をステータス欄に要約表示（取り込めた数、スキップ理由）
- 取り込み完了メッセージに保存先パス（`get_frames_dir` で取得）を含める
- Apple グループの説明文は①公式サイトを開く→②ブラウザで DMG をダウンロード（進行状況はブラウザのダウンロード欄に表示され、アプリはダウンロードしない旨を明記）→③その DMG を「DMG / PNG を取り込む」で選ぶ、の3ステップで案内する。ボタン列の先頭にグループ共通の「公式サイトを開く」ボタン（カタログ中の Apple エントリの `source_url` を使用）を置く
- 「ドロップシャドウ」トグル（section 共通）
- 永続化: `useStorage('rs-devices', [] as { id: string; variant: string | null }[])`、`useStorage('rs-frame-shadow', false)`。`list_frames` の結果に無い variant は先頭の variant に置き換える
- GIF 選択時は section を `opacity-45 pointer-events-none` にし「フレームは PNG 出力のみ」を注記（既存の比率 UI と同じ流儀）
- バリデーション: 幅もデバイスも 0 件なら「幅かデバイスを一つ以上選択してください」。デバイスのみ選択で幅 0 件は可
- ライセンス注記（section 末尾に小さく）: 「Apple のベゼルは Apple のライセンスに従いご自身の責任で使用してください。影の追加は Apple のガイドライン上は改変に当たります。Pixel のフレームは AOSP 由来（Apache 2.0）です。」

## 11. エラー処理

| 状況 | 挙動 |
|---|---|
| 選択デバイスのフレーム PNG が無い | 撮影開始前に `Err("フレームが見つかりません: iPhone 16 Pro (black-titanium)。取り込みをやり直してください")` |
| 取り込み時に寸法不一致 | そのファイルをスキップし `skipped` に `"寸法が不一致 (期待 1350x2760, 実際 …)"` |
| `hdiutil` 失敗 | stderr をそのまま `Err` に含める。マウント済みなら detach |
| カタログ JSON 不正 | `list_frames` / `capture_screenshots` が `Err("カタログの読み込みに失敗: …")`。アプリは起動する |
| 合成中の `image` エラー | そのデバイスで中断し `Err` を返す（既存の幅撮影と同じく後続は行わない） |

## 12. テスト

現状テストが無いため `#[cfg(test)]` を新設する。`cargo test` を `package.json` の `test` script に登録する。

- `compose.rs`
  - 合成用に生成した 100×200 のフレーム（(20,30) に 60×140 の透明穴）と 60×140 の単色スクショで、穴の内側がスクショ色・外側がフレーム色になる
  - スクショが穴より大きい／比率が違う場合に cover でクロップされ、出力寸法がフレームと一致する
  - shadow ON で出力寸法 = フレーム + 2·padding、本体下端のすぐ外側が黒で alpha > 0、四隅は alpha = 0
- `catalog.rs`: 同梱 `catalog.json` を読み、§5.1 の不変条件をすべて検証
- `import.rs`: `pattern` の prefix/suffix 分割と variant 抽出（`"iPhone 16 Pro - Black Titanium - Portrait.png"` → `black-titanium`）、別機種のファイルにマッチしないこと（`iPhone 16 Pro Max - …` が `iPhone 16 Pro` の pattern に掛からない）、`._` ファイルの除外、寸法不一致のスキップ
- 手動 E2E: iPhone 16 DMG の取り込み → iPhone 16 Pro（影あり／なし）と Pixel 9 / Pixel Tablet の撮影で、画面がフレームに収まり角が覆われていることを目視確認

フロントエンドは従来どおり `vue-tsc -b` の型チェックのみ。

## 13. 段階分け

1. **段階 1（Google だけで end-to-end）**: カタログ + `build-pixel-frames.sh` + 同梱 PNG + `frames/` モジュール（catalog / store / compose）+ `CaptureTarget` 化 + UI の選択・シャドウ + テスト
2. **段階 2（Apple）**: `import.rs` + `tauri-plugin-opener` + 状態バッジ・色セレクタ・取り込み UI + README / AGENTS.md 更新
3. 以降: §2.2 の項目

## 14. 依存関係とビルド設定の変更

- 追加: `tauri-plugin-opener`（Rust crate + `@tauri-apps/plugin-opener`）。`capabilities/migrated.json` に `opener:default`（`allow-open-url` 単体には URL スコープが無く `default` に含まれる `allow-default-urls` が必要）を追加（承認済み）
- `image` crate は既存の 0.24.4 のまま（PNG 読み書き・リサイズ・blur・overlay はすべて含まれる）。正規表現は `regex` crate を追加せず、`{variant}` の前後を `starts_with` / `ends_with` で判定する
- `tauri.conf.json`: `bundle.resources: ["frames/**/*"]`
- `package.json`: `"test": "cd src-tauri && cargo test"` を追加

## 15. ドキュメント

- README（日英併記）: 「Device frames」節を追加。対応機種、Apple の取り込み手順（公式ページ → DMG → 取り込み）、ライセンス注記
- AGENTS.md: 「7. デバイスフレーム合成」節を追加。カタログの不変条件、Apple 素材を同梱しない理由、合成器が常にリサイズする理由（DPR 端数と Retina 返却倍率の吸収）、シャドウのパラメータ
