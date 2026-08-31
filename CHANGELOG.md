# Changelog


## v1.2.0

[compare changes](https://github.com/annrie/ResponsiveShot/compare/v1.1.0...v1.2.0)

### 🚀 新機能

- **frames:** ✨ iPad / MacBook / iMac / Studio Display の 17 機種をカタログに追加 ([13bbd90](https://github.com/annrie/ResponsiveShot/commit/13bbd90))
- **ui:** ✨ デバイスフレームをカテゴリ別に表示し公式リンクを Design Resources に固定 ([d5f9f14](https://github.com/annrie/ResponsiveShot/commit/d5f9f14))
- ✨ iPad / MacBook / iMac / Studio Display のデバイスフレームを追加 ([#2](https://github.com/annrie/ResponsiveShot/pull/2))
- **frames:** ✨ 合成器に背景色（hex）を指定できるようにする ([ce7c321](https://github.com/annrie/ResponsiveShot/commit/ce7c321))
- **frames:** ✨ 背景色を capture_screenshots から合成器まで受け渡す ([1a507d1](https://github.com/annrie/ResponsiveShot/commit/1a507d1))
- **ui:** ✨ フレーム付き出力の背景色（透明 / 白 / 黒 / 任意）を選べるようにする ([75d18f4](https://github.com/annrie/ResponsiveShot/commit/75d18f4))
- ✨ フレーム付き出力の背景色オプション / Background color for framed output ([#3](https://github.com/annrie/ResponsiveShot/pull/3))
- **i18n:** ✨ vue-i18n を導入し ja / en ロケールと言語切替セレクトを追加 ([11102f8](https://github.com/annrie/ResponsiveShot/commit/11102f8))
- **i18n:** ✨ App.vue の UI 文字列を t() 化 ([43e9206](https://github.com/annrie/ResponsiveShot/commit/43e9206))
- **i18n:** ✨ DeviceFramePanel の UI 文字列を t() 化 ([4d3da20](https://github.com/annrie/ResponsiveShot/commit/4d3da20))
- **i18n:** ✨ 手動操作オーバーレイの文言をフロントの翻訳から渡す ([ee1013a](https://github.com/annrie/ResponsiveShot/commit/ee1013a))
- **i18n:** ✨ de / es / fr / ko / pt-BR / zh-TW の翻訳を追加 ([0584337](https://github.com/annrie/ResponsiveShot/commit/0584337))
- 🌐 UI を 8 言語に多言語化 / Internationalize the UI (8 languages) ([#5](https://github.com/annrie/ResponsiveShot/pull/5))
- **frames:** ✨ カタログに撮影用の userAgent を追加（iPhone / Pixel） ([5291610](https://github.com/annrie/ResponsiveShot/commit/5291610))
- **frames:** ✨ CaptureTarget に user_agent / touch を配線（既定 OFF） ([860398c](https://github.com/annrie/ResponsiveShot/commit/860398c))
- **frames:** ✨ モバイル UA / タッチをエミュレートするトグルを追加（既定 OFF） ([cde351e](https://github.com/annrie/ResponsiveShot/commit/cde351e))
- 📱 モバイル UA / タッチのエミュレーション（既定 OFF）/ Emulate mobile UA & touch (off by default) ([#7](https://github.com/annrie/ResponsiveShot/pull/7))

### 🐛 バグ修正

- **frames:** 🐛 カタログの vendor / category を許容値で検証する ([ff1b434](https://github.com/annrie/ResponsiveShot/commit/ff1b434))
- **frames:** 🐛 背景色の検証をデバイス撮影があるときだけ行う ([d179935](https://github.com/annrie/ResponsiveShot/commit/d179935))
- **ui:** 🐛 保存された背景色の空白と大文字小文字を正規化して透明判定を揃える ([117cc8a](https://github.com/annrie/ResponsiveShot/commit/117cc8a))
- **frames:** 🐛 スクショをフレームの穴の形でクリップし、丸い角からのはみ出しを防ぐ ([41991f0](https://github.com/annrie/ResponsiveShot/commit/41991f0))
- **frames:** 🐛 screen_mask の起点を中央近傍の非不透明画素にして、中央が不透明でもクリップを維持する ([57fb0e1](https://github.com/annrie/ResponsiveShot/commit/57fb0e1))
- **frames:** 🐛 影のシルエットにも穴マスクを適用し、丸い角の外に影が残らないようにする ([b7bd56f](https://github.com/annrie/ResponsiveShot/commit/b7bd56f))
- 🐛 スクショをフレームの穴の形でクリップして丸い角からのはみ出しを防ぐ ([#4](https://github.com/annrie/ResponsiveShot/pull/4))
- **ui:** 🐛 ダークモードでセレクトと任意色入力の文字が見えない問題を修正 ([61259d4](https://github.com/annrie/ResponsiveShot/commit/61259d4))
- **i18n:** 🌐 6 言語の翻訳をレビュー指摘に沿って修正 ([914869c](https://github.com/annrie/ResponsiveShot/commit/914869c))
- **i18n:** 🛡️ オーバーレイ注入の js_str で U+2028/U+2029 もエスケープし単体テストを追加 ([144fd61](https://github.com/annrie/ResponsiveShot/commit/144fd61))
- **ui:** 🐛 デバイスフレームの取り込みボタンにダークモードの文字色を明示し既定の枠線を除去 ([79272cf](https://github.com/annrie/ResponsiveShot/commit/79272cf))
- **i18n:** 📝 GIF のフレームレート（3 fps）と高さ自動（既定 1080px）の説明を実装に合わせて修正 ([b70b6da](https://github.com/annrie/ResponsiveShot/commit/b70b6da))
- **i18n:** ♻️ ステータス表示をキー+パラメータで保持し言語切替時に再翻訳する ([5538f5e](https://github.com/annrie/ResponsiveShot/commit/5538f5e))
- **ui:** 🎨 幅の全選択/解除ボタンのダークモード文字色を読みやすい青に変更 ([6895e2d](https://github.com/annrie/ResponsiveShot/commit/6895e2d))

### ♻️ リファクタリング

- **i18n:** 🌐 Rust のユーザー向けメッセージを英語に統一 ([78ddf42](https://github.com/annrie/ResponsiveShot/commit/78ddf42))

### 📖 ドキュメント

- 📝 デバイスフレーム v1.1 設計書（追補）を追加 ([ecdfe87](https://github.com/annrie/ResponsiveShot/commit/ecdfe87))
- 📝 Apple 機種追加（v1.1 §1）の実装計画を追加 ([0ef2fc1](https://github.com/annrie/ResponsiveShot/commit/0ef2fc1))
- 📖 Apple 追加機種の対応一覧と設計メモを追記 ([7d783da](https://github.com/annrie/ResponsiveShot/commit/7d783da))
- 📖 AGENTS.md の対応機種の記述を v1.1 に合わせる ([af16e3c](https://github.com/annrie/ResponsiveShot/commit/af16e3c))
- 📝 背景色オプション（v1.1 §2）の実装計画を追加 ([70a4cd1](https://github.com/annrie/ResponsiveShot/commit/70a4cd1))
- 📖 spec §9 の合成器の署名と手順を実装に合わせて更新 ([00d895c](https://github.com/annrie/ResponsiveShot/commit/00d895c))
- 📝 多言語化（i18n）の設計書と実装計画を追加 ([0d2be94](https://github.com/annrie/ResponsiveShot/commit/0d2be94))
- 📖 多言語対応の説明（日英）と設計制約を追記 ([f51c7d1](https://github.com/annrie/ResponsiveShot/commit/f51c7d1))
- 📝 Dynamic Island は素材が不透明に描いていると判明したため黒塗りを見送り（PR #6 クローズ） ([#6](https://github.com/annrie/ResponsiveShot/issues/6))
- 📝 UA / タッチエミュレーション（v1.1 §4）の実装計画を追加 ([e127540](https://github.com/annrie/ResponsiveShot/commit/e127540))
- 📝 §4 計画のテスト数を develop 起点（44 件）に訂正 ([5ca3565](https://github.com/annrie/ResponsiveShot/commit/5ca3565))
- 📖 UA / タッチエミュレーションの説明（日英）と設計制約を追記 ([84239f0](https://github.com/annrie/ResponsiveShot/commit/84239f0))
- 📖 UA Client Hints は上書きしない既知限界を追記 ([8ee366c](https://github.com/annrie/ResponsiveShot/commit/8ee366c))

### 🧹 ビルドプロセスまたは補助ツールの変更

- **i18n:** 🔧 check-locales の堅牢化と最終レビューの指摘を反映 ([b261801](https://github.com/annrie/ResponsiveShot/commit/b261801))

### ✅ テストの追加・修正

- **frames:** ✅ v1.1 の Apple ファイル名形式（Mac / Studio Display / iPad）の照合テストを追加 ([de9ce15](https://github.com/annrie/ResponsiveShot/commit/de9ce15))
- **frames:** ✅ UA テストを全文一致・件数固定に強化し import 機種の伝播テストを追加 ([69d510a](https://github.com/annrie/ResponsiveShot/commit/69d510a))

### ❤️ Contributors

- Annrie ([@annrie](https://github.com/annrie))

## v1.1.0

[compare changes](https://github.com/annrie/ResponsiveShot/compare/v1.0.1...v1.1.0)

### 🚀 新機能

- **frames:** ✨ フレーム合成器（cover リサイズ + オーバーレイ）を追加 ([63265d9](https://github.com/annrie/ResponsiveShot/commit/63265d9))
- **frames:** ✨ アプリ生成のドロップシャドウを合成器に追加 ([9a616d2](https://github.com/annrie/ResponsiveShot/commit/9a616d2))
- **frames:** ✨ フレームカタログの型と検証を追加 ([9840257](https://github.com/annrie/ResponsiveShot/commit/9840257))
- **frames:** ✨ Pixel フレーム生成スクリプトと v1 カタログを追加（AOSP, Apache 2.0） ([57022c9](https://github.com/annrie/ResponsiveShot/commit/57022c9))
- **frames:** ✨ フレームの保存場所解決と状態一覧を追加 ([f3553f7](https://github.com/annrie/ResponsiveShot/commit/f3553f7))
- **frames:** ✨ デバイスプリセットでの撮影とフレーム合成、list_frames コマンドを追加 ([9f5443e](https://github.com/annrie/ResponsiveShot/commit/9f5443e))
- **ui:** ✨ デバイスフレーム選択パネルとシャドウ切替を追加 ([26a8fdd](https://github.com/annrie/ResponsiveShot/commit/26a8fdd))
- **frames:** ✨ フレーム PNG の照合・寸法検証・取り込みコピーを追加 ([0f54fd9](https://github.com/annrie/ResponsiveShot/commit/0f54fd9))
- **frames:** ✨ DMG マウント付き取り込みコマンドと opener プラグインを追加 ([10f7144](https://github.com/annrie/ResponsiveShot/commit/10f7144))
- **ui:** ✨ Apple ベゼルの取り込み・公式リンク・色選択・ライセンス注記を追加 ([42dda52](https://github.com/annrie/ResponsiveShot/commit/42dda52))
- **ui:** ✨ 取り込み先の表示と Finder で開くボタンを追加 ([11b819c](https://github.com/annrie/ResponsiveShot/commit/11b819c))
- **ui:** ✨ Apple ベゼル取得手順の 3 ステップ説明と「公式サイトを開く」ボタンを追加 ([613df42](https://github.com/annrie/ResponsiveShot/commit/613df42))
- ✨ デバイスフレーム合成機能を追加 / Add device frames ([#1](https://github.com/annrie/ResponsiveShot/pull/1))

### 🐛 バグ修正

- **frames:** 🐛 store.rs の Path import をテストモジュールへ移動 ([908da20](https://github.com/annrie/ResponsiveShot/commit/908da20))
- **frames:** 🐛 取り込み走査の symlink 追従を止め、マウント済み DMG の案内と detach 再試行を追加 ([6684348](https://github.com/annrie/ResponsiveShot/commit/6684348))
- **frames:** 🐛 import_frames をメインスレッドから外して UI のフリーズを防止 ([614e701](https://github.com/annrie/ResponsiveShot/commit/614e701))
- **ui:** 🐛 DMG 選択ダイアログの拡張子フィルタを外して .dmg を選べるようにする ([7b3564b](https://github.com/annrie/ResponsiveShot/commit/7b3564b))
- **ui:** 🐛 取り込み先の取得失敗がフレーム一覧の更新を妨げないようにする ([738c66f](https://github.com/annrie/ResponsiveShot/commit/738c66f))
- **frames:** 🐛 取り込み PNG の寸法検証・原子的なコピー・Unix 限定テストの cfg・一時ディレクトリの後始末 ([8d7d3e0](https://github.com/annrie/ResponsiveShot/commit/8d7d3e0))
- **frames:** 🐛 フレーム画像を Chrome 起動前にデコードし、合成失敗時もブラウザを別スレッドで drop する ([ecb861b](https://github.com/annrie/ResponsiveShot/commit/ecb861b))

### ♻️ リファクタリング

- ♻️ キャプチャループを CaptureTarget の列挙に一般化（挙動変更なし） ([1dc7287](https://github.com/annrie/ResponsiveShot/commit/1dc7287))
- **frames:** ♻️ CaptureTarget 生成を frames/targets.rs に切り出しテストを追加 ([d6d59a8](https://github.com/annrie/ResponsiveShot/commit/d6d59a8))
- **ui:** ♻️ 「取り込み先を Finder で開く」ボタンを削除（保存先はメッセージ表示のみ） ([2b5210c](https://github.com/annrie/ResponsiveShot/commit/2b5210c))

### 📖 ドキュメント

- 📛 READMEにステータスバッジを追加 ([ddfddb9](https://github.com/annrie/ResponsiveShot/commit/ddfddb9))
- 📝 デバイスフレーム合成機能の設計書を追加 ([f0da744](https://github.com/annrie/ResponsiveShot/commit/f0da744))
- 📝 デバイスフレーム機能の実装計画（12 タスク）を追加 ([3a0a6b6](https://github.com/annrie/ResponsiveShot/commit/3a0a6b6))
- 📖 デバイスフレーム機能の使い方（日英）と設計制約を追記 ([593c93d](https://github.com/annrie/ResponsiveShot/commit/593c93d))
- 📖 GIF 時のバリデーション文言と spec の opener 権限・将来項目を更新 ([604d086](https://github.com/annrie/ResponsiveShot/commit/604d086))

### 📦 ビルド

- **deps:** 🔧 パッケージマネージャをnpmからpnpmに移行 ([ed637ca](https://github.com/annrie/ResponsiveShot/commit/ed637ca))
- **deps:** ⬆️ esbuild 0.28.2へ据え置き解除しSnyk指摘のnanoid/postcssを更新 ([d447dc9](https://github.com/annrie/ResponsiveShot/commit/d447dc9))

### ❤️ Contributors

- Annrie ([@annrie](https://github.com/annrie))

## v1.0.1

[compare changes](https://github.com/annrie/ResponsiveShot/compare/v1.0.0...v1.0.1)

### 📦 ビルド

- **deps:** ⬆️ tauri 2.11.5ほかminor/patch一括更新 ([421fbcd](https://github.com/annrie/ResponsiveShot/commit/421fbcd))
- **release:** 🔧 changelogenによるリリースフローを導入 ([4f6af0a](https://github.com/annrie/ResponsiveShot/commit/4f6af0a))

### ❤️ Contributors

- Annrie ([@annrie](https://github.com/annrie))

