# デバイスフレーム v1.1 設計書（追補）

- 日付: 2026-08-30
- 前提: v1.0 設計書 `docs/superpowers/specs/2026-08-30-device-frame-design.md`（v1.1.0 としてリリース済み）。本書は差分のみを定義し、記載のない事項は v1.0 設計書に従う
- 状態: ユーザー承認済み（チャットで設計合意、2026-08-30 16:00）
- 実装順: §1 → §2 → §3 → §4。**項目ごとに 1 本の PR**（`develop` 宛て、日英併記）

## 1. Apple 機種の追加（iPad / MacBook / iMac / Studio Display）

### 1.1 実測結果（2026-08-30、7 本の DMG を展開して中央フラッドフィルで計測）

DMG の構成は iPhone 16 と異なり、`PNG/` 直下にファイルが並ぶ（機種サブフォルダなし）。Mac 系は色の前の区切りが ` - ` ではなく空白 1 個。いずれも既存の `pattern`（`{variant}` 前後の prefix / suffix 照合）で扱えるため **取り込みコードの変更は不要**。

| id | name | category | orientation | CSS (w×h @2, mobile) | frame (w×h) | screen (x, y, w, h) | DMG / pattern |
|---|---|---|---|---|---|---|---|
| `apple-ipad-pro-11-m5-portrait` | iPad Pro 11" (M5) 縦 | tablet | portrait | 834×1210, true | 1880×2640 | 106, 110, 1668, 2420 | `Bezel-iPad-Pro-(M5).dmg` / `PNG/iPad Pro (M5) 11" - {variant} - Portrait.png` |
| `apple-ipad-pro-11-m5-landscape` | iPad Pro 11" (M5) 横 | tablet | landscape | 1210×834, true | 2640×1880 | 110, 106, 2420, 1668 | 同 / `… - Landscape.png` |
| `apple-ipad-pro-13-m5-portrait` | iPad Pro 13" (M5) 縦 | tablet | portrait | 1032×1376, true | 2300×3000 | 118, 124, 2064, 2752 | 同 / `PNG/iPad Pro (M5) 13" - {variant} - Portrait.png` |
| `apple-ipad-pro-13-m5-landscape` | iPad Pro 13" (M5) 横 | tablet | landscape | 1376×1032, true | 3000×2300 | 124, 118, 2752, 2064 | 同 / `… - Landscape.png` |
| `apple-ipad-air-11-m4-portrait` | iPad Air 11" (M4) 縦 | tablet | portrait | 820×1180, true | 1900×2620 | 130, 130, 1640, 2360 | `Bezel-iPad-Air-(M4).dmg` / `PNG/iPad Air 11" (M4) - {variant} - Portrait.png` |
| `apple-ipad-air-11-m4-landscape` | iPad Air 11" (M4) 横 | tablet | landscape | 1180×820, true | 2620×1900 | 130, 130, 2360, 1640 | 同 / `… - Landscape.png` |
| `apple-ipad-air-13-m4-portrait` | iPad Air 13" (M4) 縦 | tablet | portrait | 1024×1366, true | 2300×2980 | 126, 124, 2048, 2732 | 同 / `PNG/iPad Air 13" (M4) - {variant} - Portrait.png` |
| `apple-ipad-air-13-m4-landscape` | iPad Air 13" (M4) 横 | tablet | landscape | 1366×1024, true | 2980×2300 | 124, 126, 2732, 2048 | 同 / `… - Landscape.png` |
| `apple-ipad-mini-a17-pro-portrait` | iPad mini (A17 Pro) 縦 | tablet | portrait | 744×1133, true | 1780×2550 | 146, 142, 1488, 2266 | `Bezel-iPad-mini-(A17-Pro).dmg` / `PNG/iPad mini (A17 Pro) - {variant} - Portrait.png` |
| `apple-ipad-mini-a17-pro-landscape` | iPad mini (A17 Pro) 横 | tablet | landscape | 1133×744, true | 2550×1780 | 142, 146, 2266, 1488 | 同 / `… - Landscape.png` |
| `apple-macbook-air-13-m5` | MacBook Air 13" (M5) | laptop | landscape | 1280×832, false | 3400×2240 | 420, 288, 2560, 1664 | `Bezel-MacBook-Air-M5.dmg` / `PNG/MacBook Air M5 13-inch {variant}.png` |
| `apple-macbook-air-15-m5` | MacBook Air 15" (M5) | laptop | landscape | 1440×932, false | 3540×2300 | 329, 218, 2880, 1864 | 同 / `PNG/MacBook Air M5 15-inch {variant}.png` |
| `apple-macbook-pro-14-m5` | MacBook Pro 14" (M5) | laptop | landscape | 1512×982, false | 3860×2540 | 418, 288, 3024, 1964 | `Bezel-MacBook-Pro-M5.dmg` / `PNG/MacBook Pro M5 14-inch {variant}.png` |
| `apple-macbook-pro-16-m5` | MacBook Pro 16" (M5) | laptop | landscape | 1728×1117, false | 4260×2840 | 402, 303, 3456, 2234 | 同 / `PNG/MacBook Pro M5 16-inch {variant}.png` |
| `apple-imac-24-m4` | iMac 24" (M4) | desktop | landscape | 2240×1260, false | 4760×4050 | 140, 150, **4482**, 2520 | `Bezel-iMac-M4.dmg` / `PNG/iMac M4 24-inch {variant}.png` |
| `apple-studio-display-2026` | Studio Display (2026) | display | landscape | 2560×1440, false | 5400×4160 | 140, 140, 5120, 2880 | `Bezel-Studio-Displays.dmg` / `PNG/Studio Display 2026 {variant}.png` |
| `apple-studio-display-xdr-2026` | Studio Display XDR (2026) | display | landscape | 2560×1440, false | 5400×4160 | 140, 140, 5120, 2880 | 同 / `PNG/Studio Display XDR 2026 {variant}.png` |

DMG の URL はすべて `https://devimages-cdn.apple.com/design/resources/download/<DMG 名>`。色（variant）は取り込み時に見つかった分すべて（iPad 2〜4 色、MacBook 2〜4 色、iMac 7 色、Studio Display は `On Dark Background` / `On Light Background` の 2 種）。

- **iMac の画面矩形は 7 色の穴の和集合**（Orange だけ穴が x=142 で 2px 右にずれているため、x=140・幅 4482 にする）。余分な 2px はベゼルの下に隠れ、cover リサイズの歪みは 0.04%
- 合計 17 エントリ追加、カタログは 13 → 30 件。`catalog.rs` の件数テストを 30 に更新
- `category` の値に `laptop` / `desktop` / `display` を追加（Rust は `String` のまま、TS の union と UI ラベルを拡張）

### 1.2 UI

- Apple グループ内を **category で小見出し分け**: `phone` スマートフォン / `tablet` タブレット / `laptop` ノート PC / `desktop` デスクトップ / `display` ディスプレイ（この順）。Google グループも同じ関数で分ける
- グループの「公式サイトを開く」は **Design Resources ページ固定**（`https://developer.apple.com/design/resources/#product-bezels`）。未取り込み行の「公式」リンクは従来どおり各 DMG の直リンク
- 3 ステップ説明の「iPhone 16 の Product Bezels」を「対応機種の Product Bezels」に一般化

### 1.3 検証で確認すること

- Studio Display（5120×2880）と iMac（4480×2520）は Mac の物理画面より大きい viewport。`SetDeviceMetricsOverride` + `captureScreenshot` で欠けずに撮れることを手動 E2E で確認する。欠ける場合はエラーではなく **設計変更**（`captureBeyondViewport` の利用等）として扱い、この spec を更新する
- iPad 横向き（`mobile: true` で横長）が正しく撮れること

## 2. 背景色オプション（フレーム付き出力のみ）

- UI: デバイスフレームパネルのシャドウトグルの横に「背景」セレクト: `透明`（既定）/ `白` / `黒` / `任意`。`任意` を選ぶと `#rrggbb` の入力欄を表示。永続化 `rs-frame-bg`（`'transparent' | '#ffffff' | '#000000' | '#xxxxxx'`）
- Rust: `capture_screenshots` に `frame_background: Option<String>` を追加（`None` = 透明）。`frames::compose::parse_hex_color(&str) -> Result<Rgba<u8>, String>`（`#rgb` / `#rrggbb`、不正なら `Err("背景色の形式が不正です: …")`）。不正値は **撮影前**（`build_targets` の前）に `Err`
- `compose_frame(shot, frame, screen, shadow, background: Option<Rgba<u8>>)`: キャンバスの初期色を `background`（不透明）にする。影はその上に描く（影の alpha は従来どおり）。`None` は従来どおり透明
- 幅指定キャプチャには影響しない。ファイル名は変えない
- テスト: `parse_hex_color` の正常 / 異常、背景ありで四隅が指定色・画面部はスクショ、影ありで本体下が「背景色を暗くした色」になること

## 3. Dynamic Island の黒塗り

- カタログに任意フィールド `island: { x, y, width, height, radius }`（CSS px、画面左上原点）。iPhone 16 / 16 Plus / 16 Pro / 16 Pro Max に付ける。他機種は省略
- 値の求め方: **この Mac の iOS シミュレータで実測**。`xcrun simctl boot <iPhone 16 系>` → `simctl openurl` で白いページ（`https://example.com`）を Safari で開く → `simctl io <udid> screenshot` → 上端中央付近の黒（RGB 各 ≤ 16）画素の外接矩形を求め、幅・高さ・位置を device px ÷ 3 で CSS px に換算。`radius` は高さ ÷ 2。シミュレータが使えない場合は公開されている pt 値を使い、spec にその旨を記す
- **採用値（2026-08-31）**: iOS シミュレータは起動が終わらず実測できなかったが、取り込み済みの Apple ベゼル PNG を確認したところ **Dynamic Island は透明ではなく不透明なピルとして描かれていた**（v1.0 spec の前提が誤り）。そこで値はベゼル PNG の穴内側にある非透明画素の外接矩形（screen 原点、÷3）から実測した: iPhone 16 = x 134 / y 11 / 125 × 37.33 / r 18.67、16 Plus = 152.33 / 11.33 / 125.67 × 36.67 / r 18.33、16 Pro = 138.67 / 14.33 / 124.67 × 36 / r 18、16 Pro Max = 157.67 / 14.33 / 124.67 × 36 / r 18。黒塗りはフレームの島の内側に収まるため出力は v1 とほぼ同じで、島の縁のアンチエイリアス部分に白いページが滲むのを消す効果と、別版の素材で島が透明だった場合の保険になる
- 合成器: cover リサイズ**前**のスクショ（撮影直後の画像）に対し、`island` を `shot.width / css.width` 倍して黒の角丸矩形を描く（`compose::fill_rounded_rect(img, rect, radius, color)`、距離関数で 1px アンチエイリアス）。`island` が無ければ何もしない。**トグルは設けない**（データがあれば常時適用）
- `compose_frame` の引数に `island: Option<Rect 相当 (f32)>` を追加するのではなく、`compose_png` 側で `shot` に描いてから `compose_frame` に渡す（合成器の署名を増やさない）。描画関数は `compose.rs` に置きテストする
- テスト: `fill_rounded_rect` で矩形内が黒・角の外側が元色・境界が中間値、`island` 付きエントリのデシリアライズ

## 4. UA / タッチのエミュレーション（既定 OFF）

- UI: パネルにトグル「モバイル UA / タッチをエミュレート」（`rs-frame-emulate-mobile`、既定 `false`）。`capture_screenshots` に `emulate_mobile: bool`
- カタログ `css` に任意 `userAgent`（JSON キーは `userAgent`、Rust は `#[serde(rename = "userAgent")] user_agent: Option<String>`）:
  - iPhone 4 機種: `Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1`
  - Pixel スマホ 8 機種: `Mozilla/5.0 (Linux; Android 15; <Pixel 名>) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Mobile Safari/537.36`（`<Pixel 名>` は `Pixel 9` 等）
  - Pixel Tablet: `Mozilla/5.0 (Linux; Android 15) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36`（`Mobile` なし）
  - iPad: **UA は付けない**（実機 Safari はデスクトップ UA を名乗るため）。タッチのみ有効
  - Mac / iMac / Display: 対象外（`userAgent` なし、タッチなし）
- Rust: `CaptureTarget` に `user_agent: Option<String>` と `touch: bool` を追加（`build_targets` が `emulate_mobile && css.mobile` のとき `touch = true`、`user_agent = css.user_agent.clone()`）。ループ内で `navigate_to` の前に、`user_agent` があれば `tab.set_user_agent(ua, None, None)`、`touch` なら `Emulation::SetTouchEmulationEnabled { enabled: true, max_touch_points: Some(5) }` を送る。幅ターゲットは両方とも無効
- テスト: `build_targets` で OFF 時は `user_agent None / touch false`、ON 時は phone で `Some` / `true`、iPad で `None` / `true`、Mac で `None` / `false`

## 5. 共通

- ドキュメント: README（日英）の対応機種一覧・背景色・Island・エミュレーションの追記、AGENTS.md §7 に追記
- 各項目の PR 前に `@codex-rescue`、PR 後に GitHub Codex（`@codex review` は本文なしのコメントで再トリガー）
- リリース: 4 項目完了後に `pnpm release:minor`（v1.2.0）。手順はメモリ `release-flow-changelogen` のとおり
