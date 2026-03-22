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
   - `GIF`: 10fps（目標）での動画録画。操作が必要なアクションを残すためのモード
3. **手動インタラクション (Manual Interaction)**
   - Chrome側に専用のUIコンテナを直接注入（Inject）し、ユーザーが手動で画面上のボタンなどを操作してから「録画開始」を押せる待機機構

## 【重要】 AIエージェント開発者向けの既知の制約と特筆すべき設計
コードを編集する前に、必ず以下の**意図的な制限・アーキテクチャ設計**を理解してください。

### 1. macOSでの Chromium 落とし（Drop）デッドロック回避
`headless_chrome` には、バックグラウンドWebSocketスレッドが存在する状態でUI側Chromeが閉じられる（Dropされる）と、Rustのメインスレッドごと無限ループやデッドロックを引き起こすOS特有の既知のバグ（あるいは相性問題）があります。
* **対策**: `src-tauri/src/main.rs` 内のキャプチャループ完了時、あえて `std::mem::forget(browser);` を使い、**Browserの所有権を意図的にメモリリークさせてRust側のDropフックを無効化**しています。これによりバックエンドがフリーズせずVueへ正常にリターンできます。ブラウザウィンドウ自体はユーザーが直接「閉じる」操作を行う設計です。

### 2. RetinaディスプレイとOOM (Out Of Memory) 対策ストリーミング
MacのRetinaディスプレイでは、1440x1080pxの要求に対して実際の物理ピクセルが2880x2160pxなど（デバイスピクセル比2倍）で返却されます。
* GIFの200コマ分（20秒）の無圧縮PNGをRustの `Vec<Vec<u8>>` としてRAMに配列保持（バルク処理）しようとすると、メモリ消費が4.5GBを超え、OSのOOMキラーによってRust・Tauriプロセスごと強制キルされてしまう問題が発生しました。
* **対策**: 現在のGIFエンコーダは、1枚のスクリーンショットを撮影するごとに、その場で即座にディスク上のファイルポインタ (`&mut std::fs::File`) へ書き出しを行い、メモリから破棄する**「完全ストリーミング処理 (Sequential Disk Buffer Streaming)」**に書き換えられています。

### 3. 動的フレーム遅延 (Dynamic Framerate Parity)
ストリーミングエンコードは1枚数十ミリ〜数百ミリ秒の非同期な計算ラグが発生するため、10fpsに固定するとスローモーションの動画になってしまいます。
* **対策**: Rust側で「1コマの撮影〜圧縮（resize_exact）〜ファイル書込にかかった実時間（`elapsed_ms`）」を正確にストップウォッチで計算し、`gif::Frame` の `delay` プロパティへ動的にフィードバックしています。これにより「コマ数が荒くなっても、再生時のスピードは現実の時間と完全に同期する」パラパラ漫画方式を採用しています。

### 4. Tauriの #[command] 非同期性
画像のブロック処理がTokioの非同期ランタイムを枯渇させないよう、重いキャプチャ関数 (`capture_screenshots`, `select_element`) は敢えて `async` を外したブロッキング関数として定義しています。これによりTauriのワーカースレッドが安全に利用されます。

---
*Created by AI Agent during deep architectural refactoring to sustain memory constraints and browser lifecycle reliability.*
