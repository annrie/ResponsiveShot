//! フレームカタログ（src-tauri/frames/catalog.json）の型と検証。

use std::collections::HashSet;
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::Rect;

/// 撮影時の Chrome エミュレーション値
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CssSpec {
    pub width: u32,
    pub height: u32,
    pub dpr: f64,
    pub mobile: bool,
    /// 撮影時に名乗る UA。無い機種（iPad / Mac / Display）はエミュレーション ON でも UA を変えない
    #[serde(rename = "userAgent")]
    #[serde(default)]
    pub user_agent: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Size {
    pub width: u32,
    pub height: u32,
}

/// フレーム画像の調達元
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Source {
    /// アプリ同梱（Google Pixel）。`file` は frames/ からの相対パス
    Bundled { file: String },
    /// ユーザー取り込み（Apple）。`pattern` は `{variant}` を 1 回含むボリューム相対パス
    Import { url: String, pattern: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceEntry {
    pub id: String,
    pub vendor: String,
    pub category: String,
    pub name: String,
    pub orientation: String,
    pub css: CssSpec,
    pub frame: Size,
    pub screen: Rect,
    pub source: Source,
}

pub fn parse_catalog(json: &str) -> Result<Vec<DeviceEntry>, String> {
    let entries: Vec<DeviceEntry> =
        serde_json::from_str(json).map_err(|e| format!("Failed to load the frame catalog: {}", e))?;
    validate(&entries)?;
    Ok(entries)
}

pub fn load_catalog(path: &Path) -> Result<Vec<DeviceEntry>, String> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| format!("Failed to load the frame catalog: {}: {}", path.display(), e))?;
    parse_catalog(&json)
}

/// spec §5.1 の不変条件。同梱ファイルの存在と寸法はカタログ自体のテスト（Task 4）で確認する。
/// vendor は `apple` / `google`、category は `phone` / `tablet` / `laptop` / `desktop` / `display` のみ許容する
pub fn validate(entries: &[DeviceEntry]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for e in entries {
        if !seen.insert(e.id.as_str()) {
            return Err(format!("Duplicate catalog id: {}", e.id));
        }
        if e.id.is_empty()
            || !e.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(format!("Catalog id must be lowercase letters, digits and hyphens: {:?}", e.id));
        }
        const VENDORS: [&str; 2] = ["apple", "google"];
        const CATEGORIES: [&str; 5] = ["phone", "tablet", "laptop", "desktop", "display"];
        if !VENDORS.contains(&e.vendor.as_str()) {
            return Err(format!("{}: invalid vendor: {:?}", e.id, e.vendor));
        }
        if !CATEGORIES.contains(&e.category.as_str()) {
            return Err(format!("{}: invalid category: {:?}", e.id, e.category));
        }
        if e.screen.right() > e.frame.width || e.screen.bottom() > e.frame.height {
            return Err(format!("{}: screen rect exceeds the frame", e.id));
        }
        if let Source::Import { pattern, .. } = &e.source {
            if pattern.matches("{variant}").count() != 1 {
                return Err(format!(
                    "{}: pattern must contain {{variant}} exactly once",
                    e.id
                ));
            }
        }
    }
    Ok(())
}

pub fn find<'a>(entries: &'a [DeviceEntry], id: &str) -> Option<&'a DeviceEntry> {
    entries.iter().find(|e| e.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"[
      { "id": "google-pixel-9", "vendor": "google", "category": "phone", "name": "Pixel 9", "orientation": "portrait",
        "css": { "width": 412, "height": 923, "dpr": 2.625, "mobile": true },
        "frame": { "width": 1198, "height": 2531 },
        "screen": { "x": 55, "y": 58, "width": 1080, "height": 2424 },
        "source": { "kind": "bundled", "file": "google/pixel_9.png" } },
      { "id": "apple-iphone-16-pro", "vendor": "apple", "category": "phone", "name": "iPhone 16 Pro", "orientation": "portrait",
        "css": { "width": 402, "height": 874, "dpr": 3.0, "mobile": true },
        "frame": { "width": 1350, "height": 2760 },
        "screen": { "x": 72, "y": 69, "width": 1206, "height": 2622 },
        "source": { "kind": "import", "url": "https://example.com/Bezel-iPhone-16.dmg",
                    "pattern": "PNG/iPhone 16 Pro/iPhone 16 Pro - {variant} - Portrait.png" } }
    ]"#;

    #[test]
    fn parses_both_source_kinds() {
        let entries = parse_catalog(SAMPLE).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].source, Source::Bundled { file: "google/pixel_9.png".into() });
        assert!(matches!(&entries[1].source, Source::Import { pattern, .. } if pattern.contains("{variant}")));
        assert_eq!(find(&entries, "apple-iphone-16-pro").unwrap().css.dpr, 3.0);
        assert!(find(&entries, "nope").is_none());
    }

    #[test]
    fn rejects_duplicate_id() {
        let mut e = parse_catalog(SAMPLE).unwrap();
        e[1].id = e[0].id.clone();
        assert!(validate(&e).unwrap_err().contains("Duplicate"));
    }

    #[test]
    fn rejects_bad_id_chars() {
        let mut e = parse_catalog(SAMPLE).unwrap();
        e[0].id = "Pixel 9".into();
        assert!(validate(&e).unwrap_err().contains("lowercase"));
    }

    #[test]
    fn rejects_unknown_vendor() {
        let mut e = parse_catalog(SAMPLE).unwrap();
        e[0].vendor = "samsung".into();
        assert!(validate(&e).unwrap_err().contains("invalid vendor"));
    }

    #[test]
    fn rejects_unknown_category() {
        let mut e = parse_catalog(SAMPLE).unwrap();
        e[1].category = "watch".into();
        assert!(validate(&e).unwrap_err().contains("invalid category"));
    }

    #[test]
    fn rejects_screen_outside_frame() {
        let mut e = parse_catalog(SAMPLE).unwrap();
        e[0].screen.x = 200; // 200 + 1080 > 1198
        assert!(validate(&e).unwrap_err().contains("exceeds the frame"));
    }

    #[test]
    fn rejects_pattern_without_variant() {
        let mut e = parse_catalog(SAMPLE).unwrap();
        e[1].source = Source::Import { url: "u".into(), pattern: "PNG/x.png".into() };
        assert!(validate(&e).unwrap_err().contains("{variant}"));
    }

    #[test]
    fn reports_invalid_json() {
        assert!(parse_catalog("[{").unwrap_err().starts_with("Failed to load the frame catalog"));
    }

    /// 同梱カタログそのもの: 30 件、不変条件を満たし、bundled の PNG が存在して frame 寸法と一致する
    #[test]
    fn bundled_catalog_is_valid_and_bundled_pngs_match_frame_size() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("frames");
        let entries = load_catalog(&root.join("catalog.json")).expect("frames/catalog.json");
        assert_eq!(entries.len(), 30);
        for e in &entries {
            if let Source::Bundled { file } = &e.source {
                let path = root.join(file);
                let (w, h) = image::image_dimensions(&path)
                    .unwrap_or_else(|err| panic!("{}: {}", path.display(), err));
                assert_eq!((w, h), (e.frame.width, e.frame.height), "{}", e.id);
            }
        }
    }

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
        const IPHONE_UA: &str = "Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Mobile/15E148 Safari/604.1";
        const TABLET_UA: &str = "Mozilla/5.0 (Linux; Android 15) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Safari/537.36";
        let (mut iphones, mut pixels, mut tablets) = (0, 0, 0);
        for e in &entries {
            let ua = e.css.user_agent.as_deref();
            match (e.vendor.as_str(), e.category.as_str()) {
                // iPhone: iOS Safari の UA（全文一致）
                ("apple", "phone") => {
                    assert_eq!(ua, Some(IPHONE_UA), "{}", e.id);
                    iphones += 1;
                }
                // Pixel スマホ: Android Chrome の UA（機種名入り、全文一致）
                ("google", "phone") => {
                    let expected = format!("Mozilla/5.0 (Linux; Android 15; {}) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/128.0.0.0 Mobile Safari/537.36", e.name);
                    assert_eq!(ua, Some(expected.as_str()), "{}", e.id);
                    pixels += 1;
                }
                // Pixel Tablet: Mobile トークンなしの Android Chrome（全文一致）
                ("google", "tablet") => {
                    assert_eq!(ua, Some(TABLET_UA), "{}", e.id);
                    tablets += 1;
                }
                // iPad はデスクトップ UA を名乗るので付けない。Mac / iMac / Display も対象外
                _ => assert_eq!(ua, None, "{}", e.id),
            }
        }
        assert_eq!((iphones, pixels, tablets), (4, 8, 1), "UA を持つ件数");
    }
}
