# ResponsiveShot 仕様・アーキテクチャ要件 (AI Agents Context)

本ドキュメントは、AIアシスタントやエージェントが本プロジェクト (`ResponsiveShot`) のコードベースを読み解き、将来的な機能拡張や保守を行うためのコンテキスト・仕様書です。

## 概要 (Project Overview)
**ResponsiveShot** は、指定したURLのWebページに対し、任意の複数画面幅（320px〜1600pxなど）で一括スクリーンショットや操作アニメーション（GIF）を撮影できるデスクトップアプリケーションです。レスポンシブデザインの確認や、操作モーダルのエビデンス収録などに特化しています。

## 技術スタック (Tech Stack)
* **フロントエンド**: Vue 3 + Vite + TypeScript
* **スタイリング**: UnoCSS (`dark: class` によるダークモード対応)
* **状態管理**: VueUse (`useStorage` を全設定項目の永続化に利用, `useColorMode`)
* **バックエンド**: Tauri (v2) + Rust
* **ブラウザ制御**: `headless_chrome` クレート (表示あり・非ヘッドレス駆動)
* **画像・動画処理**: `image` クレート, `gif` クレート

## 主要機能 (Core Features)
1. **キャプチャモード (Capture Modes)**
   - `fullpage`: ページ全体を下部まで取得する
   - `viewport`: ファーストビューのみをそのまま取得する
   - `element`: UIから起動する「要素選択モード」によって注入されるJSで、ユーザーが直接指定したDOM要素の境界矩形（Bounding Box）だけを精密に切り抜く
2. **出力フォーマット (Output Formats)**
   - `PNG`: 各幅ごとに静止画を1枚ずつ書き出す標準機能
   - `GIF`: 3fps固定での動画録画。操作が必要なアクションを残すためのモード。出力は最大800px幅に制限される（エンコード速度とファイルサイズの最適化のため）
3. **手動インタラクション (Manual Interaction)**
   - Chrome側に専用のUIコンテナを直接注入（Inject）し、ユーザーが手動で画面上のボタンなどを操作してから「録画開始」を押せる待機機構
4. **デバイスフレーム (Device Frames)**
   - カタログ `src-tauri/frames/catalog.json` に登録した端末（Apple iPhone 16 系 4 機種・Google Pixel 9/10 系 + Pixel Tablet）を選ぶと、その CSS 寸法・DPR・mobile で viewport 撮影し、Rust 側 `frames::compose` でベゼル PNG に合成して保存する。ドロップシャドウはアプリが生成する
   - Google 分は同梱（AOSP, Apache 2.0）、Apple 分はユーザーが公式 DMG を取り込む（`frames::import`、`hdiutil attach` を使用）

## 【重要】 AIエージェント開発者向けの既知の制約と特筆すべき設計
コードを編集する前に、必ず以下の**意図的な制限・アーキテクチャ設計**を理解してください。

### 1. macOSでの Chromium 落とし（Drop）デッドロック回避
`headless_chrome` には、バックグラウンドWebSocketスレッドが存在する状態でUI側Chromeが閉じられる（Dropされる）と、Rustのメインスレッドごと無限ループやデッドロックを引き起こすOS特有の既知のバグ（あるいは相性問題）があります。
* **対策**: `src-tauri/src/main.rs` 内のキャプチャループ完了時、`std::thread::spawn(move || { drop(browser); });` で**別スレッドでブラウザをdrop**しています。これによりメインスレッドをブロックせず、かつChromeプロセスのリークも防止しています。

### 2. RetinaディスプレイとOOM (Out Of Memory) 対策ストリーミング
MacのRetinaディスプレイでは、1440x1080pxの要求に対して実際の物理ピクセルが2880x2160pxなど（デバイスピクセル比2倍）で返却されます。
* GIFの200コマ分（20秒）の無圧縮PNGをRustの `Vec<Vec<u8>>` としてRAMに配列保持（バルク処理）しようとすると、メモリ消費が4.5GBを超え、OSのOOMキラーによってRust・Tauriプロセスごと強制キルされてしまう問題が発生しました。
* **対策**: 現在のGIFエンコーダは、1枚のスクリーンショットを撮影するごとに、その場で即座にディスク上のファイルポインタ (`&mut std::fs::File`) へ書き出しを行い、メモリから破棄する**「完全ストリーミング処理 (Sequential Disk Buffer Streaming)」**に書き換えられています。

### 3. 固定フレームレートと最大幅制限 (Fixed Framerate & Max Width Cap)
高解像度ビューポート（1024px以上）でのGIFエンコードは、スクリーンショット取得 + RGBA変換 + 色量子化が非常に重く、リアルタイム録画が困難です。
* **対策1 (フレームレート)**: 3FPS固定（330ms間隔）でフレームを撮影し、`frame.delay = 33`（GIF仕様の10ms単位）を設定。撮影+エンコードが330ms未満で完了した場合は `sleep` で間隔を調整し、安定した再生速度を維持しています。
* **対策2 (最大幅制限)**: GIFフレームの幅を最大800pxに制限し、それを超える場合はアスペクト比を保って`FilterType::Triangle`（バイリニア補間）でリサイズしています。これによりエンコード対象のピクセル数を抑え、1440px幅でも安定したフレーム数を確保しています。375〜768px幅はリサイズなしでそのまま出力されます。

### 4. Tauriの #[command] 非同期性
画像のブロック処理がTokioの非同期ランタイムを枯渇させないよう、重いキャプチャ関数 (`capture_screenshots`, `select_element`) は敢えて `async` を外したブロッキング関数として定義しています。これによりTauriのワーカースレッドが安全に利用されます。

### 5. 待機時間設定の共通化
待機時間（`startDelay`）はGIF録画時だけでなく、PNGキャプチャ時にもページ描画の待機に使われます。フロントエンドではGIF専用設定とは別の共通セクションとして表示し、どちらのモードでも調整可能にしています。

### 6. GIF録画中の進捗表示
録画中はChrome側に注入したUIコンテナ（`id="rs-recorder-ui"`）の最初の`div`要素のテキストを直接更新し、残り秒数と完了コマ数を表示します。XSS防止のため `textContent` を使用しています。

### 7. デバイスフレーム合成の設計制約
- **Apple のベゼル画像はリポジトリにもアプリにも含めない。** App Store Marketing Artwork License が再配布を認めていないため。カタログにはメタデータ（画面矩形・公式 DL URL・ファイル名パターン）だけを持ち、画像はユーザーが取り込む
- **カタログの不変条件**（`frames::catalog::validate` とテストで検証）: `id` は一意で英小文字・数字・ハイフンのみ、`screen` は `frame` に内包、`import` の `pattern` は `{variant}` をちょうど 1 回含む、`bundled` の PNG は存在して `frame` 寸法と一致
- **合成器は常に画面矩形へ cover リサイズする。** Pixel 9 の DPR 2.625 のような端数（412×2.625 = 1081.5）や Retina での返却倍率のブレを吸収するため。角丸クリップはしない（フレーム側の角が不透明でスクショの角を覆う）
- **シャドウのパラメータは固定**: `sigma = 0.015 × 幅`、`offset_y = 0.015 × 高さ`、不透明度 0.35、パディング `3σ + offset_y`。ぼかしは 1/4 縮小で行う（フルサイズだと 1470×3000 で数秒かかる）
- **幅指定の出力は変えない。** `CaptureTarget` の幅ターゲットは `dpr 1.0 / mobile false` 固定で、ファイル名も従来どおり
- **デバイスターゲットは viewport / PNG 固定。** GIF と同時指定は Rust 側で `Err` にする（フロントは GIF 選択時に `devices: []` を送る）
- Apple の Product Bezels PNG は Dynamic Island 部分も透明なので、ページ内容が透けて見える（v1 仕様。黒塗りは将来拡張）

---
*Updated after GIF recording stabilization: frame rate fix, max width cap, browser drop threading, and UI improvements.*
