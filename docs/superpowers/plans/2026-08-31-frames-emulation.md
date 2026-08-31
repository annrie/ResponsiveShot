# デバイスフレーム v1.1 §4 UA / タッチのエミュレーション 実装計画

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** デバイスフレーム撮影時に、機種相応のモバイル UA とタッチイベントをエミュレートするトグル（既定 OFF）を追加し、UA で出し分けるサイトを実機どおりに撮れるようにする。

**Architecture:** カタログ `css` に任意 `userAgent` を持たせ、`build_targets(emulate_mobile)` が `CaptureTarget.user_agent / touch` に落とす。`main.rs` の撮影ループで `navigate_to` の前に `tab.set_user_agent` と `Emulation::SetTouchEmulationEnabled` を送る。UI は DeviceFramePanel のシャドウトグルの隣にチェックボックスを 1 つ。

**Tech Stack:** Rust（headless_chrome 1.0.22 の `Tab::set_user_agent(&str, Option<&str>, Option<&str>)` と `Emulation::SetTouchEmulationEnabled { enabled, max_touch_points: Option<JsUInt> }`）、Vue 3 + vue-i18n（新キーは 8 ロケール全部に追加）。

**Spec:** `docs/superpowers/specs/2026-08-30-device-frame-v1.1-design.md` §4

## Global Constraints

- 既定 OFF。OFF のとき全ターゲットが `user_agent: None / touch: false` で、**出力・挙動は従来と完全に同じ**
- 幅指定ターゲット（デバイスでない方）は ON でも常に `None / false`
- UA はカタログ `css.userAgent`（JSON キーは `userAgent`、Rust は `#[serde(rename = "userAgent")] #[serde(default)] pub user_agent: Option<String>`）にだけ由来する。コード側に UA 文字列をハードコードしない
- タッチは `emulate_mobile && css.mobile` で決まる（`userAgent` の有無とは独立。iPad = UA なし + タッチあり）
- `SetTouchEmulationEnabled` は `enabled: true, max_touch_points: Some(5)`。UA は `tab.set_user_agent(ua, None, None)`。どちらも `set_viewport_metrics` より前に送る
- 新しい UI 文字列は `frames.emulateMobile` の 1 キーのみ。**8 ロケール全部**（ja/en/de/es/fr/ko/pt-BR/zh-TW）の `frames` セクション、`"shadow"` の直後に追加（`node scripts/check-locales.mjs` が `8 locales, 118 keys OK` になること）
- 新規依存なし、`let ... else` なし、`#[allow(dead_code)]` なし。`pnpm test` 合格、`cargo check` 警告 0。ユーザー向けエラー文言は英語、コードコメントは日本語
- コミットメッセージは `<type>(scope): <emoji> 日本語`。`.superpowers/`、`dist/`、`dist-release/` はコミットしない
- テスト数の推移: 現在 44（develop 起点。island 分は落とした）→ Task 1 で +2 = 46 → Task 2 で +3 = 49

## File Structure

| パス | 責務 |
|---|---|
| `src-tauri/src/frames/catalog.rs` | `CssSpec.user_agent`、テスト |
| `src-tauri/frames/catalog.json` | iPhone 4 + Pixel スマホ 8 + Pixel Tablet に `userAgent` |
| `src-tauri/src/frames/targets.rs` | `CaptureTarget.user_agent / touch`、`build_targets` の `emulate_mobile` 引数、テスト |
| `src-tauri/src/main.rs` | `capture_screenshots` の `emulate_mobile` 引数、ループでの UA / タッチ適用 |
| `src/components/DeviceFramePanel.vue`, `src/App.vue` | トグル UI、`rs-frame-emulate-mobile`、invoke 引数 |
| `src/locales/*.json`（8 ファイル） | `frames.emulateMobile` |
| `README.md`, `AGENTS.md` | 説明（日英）と設計制約 |

---

### Task 1: カタログの `userAgent`

**Files:**
- Modify: `src-tauri/src/frames/catalog.rs`（`CssSpec` のフィールド + テスト 2 件）
- Modify: `src-tauri/frames/catalog.json`（13 件に `userAgent`）

**Interfaces:**
- Produces: `CssSpec.user_agent: Option<String>`（JSON キー `userAgent`、省略時 `None`）

- [ ] **Step 1: 失敗するテストを書く**

`catalog.rs` の `mod tests` に追加:

```rust
    #[test]
    fn user_agent_is_optional_and_deserializes() {
        let entries = parse_catalog(SAMPLE).unwrap();
        assert_eq!(entries[0].css.user_agent, None, "SAMPLE には userAgent が無いので None");
        let json = SAMPLE.replacen(
            r#""css": { "width": 412,"#,
            r#""css": { "userAgent": "TestUA/1.0", "width": 412,"#,
            1,
        );
        let entries = parse_catalog(&json).unwrap();
        assert_eq!(entries[0].css.user_agent.as_deref(), Some("TestUA/1.0"));
    }

    #[test]
    fn bundled_catalog_user_agents_follow_the_device_rules() {
        let entries = load_catalog(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/frames/catalog.json"))).unwrap();
        for e in &entries {
            let ua = e.css.user_agent.as_deref();
            match (e.vendor.as_str(), e.category.as_str()) {
                // iPhone: iOS Safari の UA
                ("apple", "phone") => {
                    let ua = ua.expect(&e.id);
                    assert!(ua.contains("iPhone OS 18_0") && ua.contains("Mobile/15E148"), "{}: {}", e.id, ua);
                }
                // Pixel スマホ: Android Chrome の UA（機種名と Mobile トークンを含む）
                ("google", "phone") => {
                    let ua = ua.expect(&e.id);
                    assert!(ua.contains(&format!("; {})", e.name)) && ua.contains("Mobile Safari"), "{}: {}", e.id, ua);
                }
                // Pixel Tablet: Android Chrome だが Mobile トークンなし
                ("google", "tablet") => {
                    let ua = ua.expect(&e.id);
                    assert!(ua.contains("Android 15") && !ua.contains("Mobile"), "{}: {}", e.id, ua);
                }
                // iPad はデスクトップ UA を名乗るので付けない。Mac / iMac / Display も対象外
                _ => assert_eq!(ua, None, "{}", e.id),
            }
        }
    }
```

- [ ] **Step 2: 失敗を確認**

Run: `cd src-tauri && cargo test frames::catalog 2>&1 | tail -5`
Expected: コンパイルエラー（`user_agent` が `CssSpec` に無い）

- [ ] **Step 3: 実装**

`CssSpec` に追加（`mobile` の後ろ）:

```rust
    /// 撮影時に名乗る UA。無い機種（iPad / Mac / Display）はエミュレーション ON でも UA を変えない
    #[serde(rename = "userAgent")]
    #[serde(default)]
    pub user_agent: Option<String>,
```

`catalog.json` の `css` オブジェクトに `"userAgent"` を追加（`"mobile"` の後ろ、13 件）:

- iPhone 4 件（apple-iphone-16 / -plus / -pro / -pro-max）共通:
  `"userAgent": "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1"`
- Pixel スマホ 8 件は `<name>` をエントリの `name`（Pixel 9 / Pixel 9 Pro / Pixel 9 Pro XL / Pixel 9a / Pixel 10 / Pixel 10 Pro / Pixel 10 Pro XL / Pixel 10a）に置き換えて:
  `"userAgent": "Mozilla/5.0 (Linux; Android 15; <name>) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Mobile Safari/537.36"`
- google-pixel-tablet（`Mobile` なし）:
  `"userAgent": "Mozilla/5.0 (Linux; Android 15) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36"`
- iPad 10 件・MacBook 4 件・iMac・Display 2 件には付けない

- [ ] **Step 4: 合格を確認**

Run: `cd src-tauri && cargo test 2>&1 | tail -3 && cargo check 2>&1 | grep -c warning`
Expected: `46 passed`、警告 `0`

- [ ] **Step 5: コミット**

```bash
git add src-tauri/src/frames/catalog.rs src-tauri/frames/catalog.json
git commit -m "feat(frames): ✨ カタログに撮影用の userAgent を追加（iPhone / Pixel）"
```

---

### Task 2: `CaptureTarget` への配線

**Files:**
- Modify: `src-tauri/src/frames/targets.rs`（構造体 2 フィールド、`build_targets` の引数、SAMPLE の拡張、テスト 3 件）

**Interfaces:**
- Consumes: `CssSpec.user_agent`（Task 1）
- Produces: `CaptureTarget.user_agent: Option<String>` / `CaptureTarget.touch: bool`、`build_targets(widths, viewport_height, capture_height, devices, frame_shadow, frame_background, duration, emulate_mobile: bool, frames)`（`emulate_mobile` は `duration` の後・`frames` の前）

- [ ] **Step 1: 失敗するテストを書く**

`targets.rs` のテスト用 `SAMPLE` を拡張する。pixel_9 の `css` に `"userAgent": "Mozilla/5.0 (Linux; Android 15; Pixel 9) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Mobile Safari/537.36"` を追加し、さらに 2 エントリを末尾に追加:

```json
      { "id": "test-tablet", "vendor": "google", "category": "tablet", "name": "Test Tablet", "orientation": "landscape",
        "css": { "width": 1280, "height": 800, "dpr": 2.0, "mobile": true },
        "frame": { "width": 2600, "height": 1700 },
        "screen": { "x": 20, "y": 50, "width": 2560, "height": 1600 },
        "source": { "kind": "bundled", "file": "google/test_tablet.png" } },
      { "id": "test-laptop", "vendor": "apple", "category": "laptop", "name": "Test Laptop", "orientation": "landscape",
        "css": { "width": 1440, "height": 900, "dpr": 2.0, "mobile": false },
        "frame": { "width": 3000, "height": 1900 },
        "screen": { "x": 60, "y": 50, "width": 2880, "height": 1800 },
        "source": { "kind": "bundled", "file": "google/test_laptop.png" } }
```

テスト 3 件を追加（既存テストと同じ `roots(tag)` / `touch(...)` を使う）:

```rust
    #[test]
    fn emulation_off_keeps_user_agent_and_touch_off() {
        let entries = catalog::parse_catalog(SAMPLE).unwrap();
        let r = roots("emu-off");
        touch(&r.bundled.join("google/pixel_9.png"), 1198, 2531);
        let devices = [DeviceSelection { id: "google-pixel-9".into(), variant: None }];
        let targets = build_targets(&[1024], None, 1080, &devices, false, None, 0, false, Some((&entries, &r))).unwrap();
        for t in &targets {
            assert_eq!(t.user_agent, None, "{}", t.label);
            assert!(!t.touch, "{}", t.label);
        }
    }

    #[test]
    fn emulation_on_sets_user_agent_and_touch_per_device() {
        let entries = catalog::parse_catalog(SAMPLE).unwrap();
        let r = roots("emu-on");
        touch(&r.bundled.join("google/pixel_9.png"), 1198, 2531);
        touch(&r.bundled.join("google/test_tablet.png"), 2600, 1700);
        touch(&r.bundled.join("google/test_laptop.png"), 3000, 1900);
        let devices = [
            DeviceSelection { id: "google-pixel-9".into(), variant: None },
            DeviceSelection { id: "test-tablet".into(), variant: None },
            DeviceSelection { id: "test-laptop".into(), variant: None },
        ];
        let targets = build_targets(&[], None, 1080, &devices, false, None, 0, true, Some((&entries, &r))).unwrap();
        assert!(targets[0].user_agent.as_deref().unwrap().contains("Pixel 9"), "スマホは UA あり");
        assert!(targets[0].touch, "スマホはタッチあり");
        assert_eq!(targets[1].user_agent, None, "userAgent の無いタブレットは UA を変えない");
        assert!(targets[1].touch, "mobile:true ならタッチはあり（iPad 相当）");
        assert_eq!(targets[2].user_agent, None, "ラップトップは UA なし");
        assert!(!targets[2].touch, "mobile:false ならタッチなし");
    }

    #[test]
    fn emulation_on_leaves_width_targets_untouched() {
        let targets = build_targets(&[375, 1024], None, 1080, &[], false, None, 0, true, None).unwrap();
        for t in &targets {
            assert_eq!(t.user_agent, None, "{}", t.label);
            assert!(!t.touch, "{}", t.label);
        }
    }
```

- [ ] **Step 2: 失敗を確認**

Run: `cd src-tauri && cargo test frames::targets 2>&1 | tail -5`
Expected: コンパイルエラー（引数の数・フィールドが無い）

- [ ] **Step 3: 実装**

`CaptureTarget` に追加（`frame` の前）:

```rust
    /// エミュレーション ON かつカタログに userAgent がある場合だけ Some
    pub user_agent: Option<String>,
    /// エミュレーション ON かつ css.mobile のときタッチイベントを有効にする
    pub touch: bool,
```

`build_targets` の署名に `emulate_mobile: bool` を追加（`duration: u32,` の直後）。幅ターゲットの生成箇所には `user_agent: None, touch: false,` を、デバイスターゲットの生成箇所には

```rust
                user_agent: if emulate_mobile { entry.css.user_agent.clone() } else { None },
                touch: emulate_mobile && entry.css.mobile,
```

を追加する。既存テストの `build_targets(...)` 呼び出し（8 箇所前後）には `duration` の次の引数として `false` を挿入して回る。

- [ ] **Step 4: 合格を確認**

Run: `cd src-tauri && cargo test 2>&1 | tail -3`
Expected: コンパイルエラーが `main.rs` の `build_targets` 呼び出しに残る場合は、この時点では `main.rs` の呼び出しに `false` を仮で入れておく（Task 3 で本配線に置き換える）。`49 passed`、警告 0

- [ ] **Step 5: コミット**

```bash
git add src-tauri/src/frames/targets.rs src-tauri/src/main.rs
git commit -m "feat(frames): ✨ CaptureTarget に user_agent / touch を配線（既定 OFF）"
```

---

### Task 3: 撮影ループへの適用とトグル UI

**Files:**
- Modify: `src-tauri/src/main.rs`（`capture_screenshots` の引数、ループでの適用）
- Modify: `src/App.vue`（`rs-frame-emulate-mobile`、invoke 引数、`v-model`）
- Modify: `src/components/DeviceFramePanel.vue`（トグル）
- Modify: `src/locales/{ja,en,de,es,fr,ko,pt-BR,zh-TW}.json`（`frames.emulateMobile`）

**Interfaces:**
- Consumes: `build_targets(..., duration, emulate_mobile, frames)`（Task 2）、`Tab::set_user_agent`、`Emulation::SetTouchEmulationEnabled`

- [ ] **Step 1: Rust 側**

`capture_screenshots` の引数に `emulate_mobile: bool,` を追加（`frame_background: Option<String>,` の直後）。Task 2 で仮置きした `build_targets(..., false, ...)` の `false` を `emulate_mobile` に置き換える。撮影ループの `set_viewport_metrics(&tab, ...)` の直前に追加:

```rust
        // UA / タッチのエミュレーション（デバイスターゲットでトグル ON のときだけ値が入る）
        if let Some(ua) = &target.user_agent {
            tab.set_user_agent(ua, None, None).map_err(|e| e.to_string())?;
        }
        if target.touch {
            tab.call_method(Emulation::SetTouchEmulationEnabled { enabled: true, max_touch_points: Some(5) })
                .map_err(|e| e.to_string())?;
        }
```

- [ ] **Step 2: フロント側**

- `src/App.vue`: `const emulateMobile = useStorage('rs-frame-emulate-mobile', false)`（`frameShadow` の隣）、invoke の引数に `emulateMobile: emulateMobile.value,`（`frameBackground` の隣）、`<DeviceFramePanel>` に `v-model:emulate-mobile="emulateMobile"` を追加
- `src/components/DeviceFramePanel.vue`: `const emulateMobile = defineModel<boolean>('emulateMobile', { required: true })` を `shadow` の隣に追加し、シャドウトグルの `<label>` の直後に同じ体裁で:

```html
        <label class="flex items-center gap-2 cursor-pointer">
          <input type="checkbox" v-model="emulateMobile" class="text-blue-500 rounded" />
          {{ t('frames.emulateMobile') }}
        </label>
```

- 8 ロケールの `frames` セクション、`"shadow"` の直後に `emulateMobile` を追加:

| locale | 値 |
|---|---|
| ja | `モバイル UA / タッチをエミュレート` |
| en | `Emulate mobile UA / touch` |
| de | `Mobile UA / Touch emulieren` |
| es | `Emular UA móvil / táctil` |
| fr | `Émuler UA mobile / tactile` |
| ko | `모바일 UA / 터치 에뮬레이션` |
| pt-BR | `Emular UA móvel / toque` |
| zh-TW | `模擬行動版 UA / 觸控` |

- [ ] **Step 3: 検証**

Run: `node scripts/check-locales.mjs && pnpm run build && cd src-tauri && cargo test 2>&1 | tail -3 && cargo check 2>&1 | grep -c warning`
Expected: `check-locales: 8 locales, 118 keys OK`、build 成功、`49 passed`、警告 `0`

- [ ] **Step 4: コミット**

```bash
git add src-tauri/src/main.rs src/App.vue src/components/DeviceFramePanel.vue src/locales
git commit -m "feat(frames): ✨ モバイル UA / タッチをエミュレートするトグルを追加（既定 OFF）"
```

---

### Task 4: ドキュメント

**Files:**
- Modify: `README.md`（Device frames 節、日英 1 行ずつ）
- Modify: `AGENTS.md`（§7 に 1 箇条）

- [ ] **Step 1: README**

Device frames 節の英語リスト末尾に:

```markdown
- Optional "Emulate mobile UA / touch" toggle (off by default): captures with the device's user agent (iPhone Safari / Pixel Chrome) and touch events enabled. iPads keep the desktop UA (as real ones do) but get touch; Macs and displays are unaffected.
```

日本語リスト末尾に:

```markdown
- 「モバイル UA / タッチをエミュレート」トグル（既定 OFF）: 機種相応の UA（iPhone Safari / Pixel Chrome）とタッチイベントで撮影します。iPad は実機どおりデスクトップ UA のままタッチのみ有効。Mac・ディスプレイは対象外です。
```

- [ ] **Step 2: AGENTS.md §7 末尾に**

```markdown
- **UA / タッチ**: カタログ `css.userAgent`（任意）と `css.mobile` から `build_targets(emulate_mobile)` が `CaptureTarget.user_agent / touch` を決める（既定 OFF、幅ターゲットは常に無効）。適用は撮影ループの `set_viewport_metrics` 前（`tab.set_user_agent` + `Emulation::SetTouchEmulationEnabled`）。UA 文字列はコードに置かずカタログにだけ持たせる
```

- [ ] **Step 3: 確認とコミット**

Run: `git diff --stat` → 2 ファイルのみ

```bash
git add README.md AGENTS.md
git commit -m "docs: 📖 UA / タッチエミュレーションの説明（日英）と設計制約を追記"
```

---

## 完了後

1. ブランチ全体の最終レビュー → `@codex-rescue` → push → PR（`feature/frames-emulation` → develop、日英併記）→ 本文なし `@codex review` → 指摘対応
2. 手動 E2E: トグル ON + UA で出し分けるサイト（例: スマホでレイアウトが変わるサイト）を iPhone 16 Pro で撮ると実機相当になる / OFF なら従来どおり / iPad は UA が変わらない
3. マージ → main ff 同期 → ブランチ削除 → 台帳削除 → **v1.2.0 リリース**（i18n + §4。手順はメモリ `release-flow-changelogen`、zip は `--norsrc`）
