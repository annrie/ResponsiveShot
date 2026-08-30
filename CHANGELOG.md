# Changelog


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

