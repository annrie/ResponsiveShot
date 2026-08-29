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
        serde_json::from_str(json).map_err(|e| format!("カタログの読み込みに失敗: {}", e))?;
    validate(&entries)?;
    Ok(entries)
}

pub fn load_catalog(path: &Path) -> Result<Vec<DeviceEntry>, String> {
    let json = std::fs::read_to_string(path)
        .map_err(|e| format!("カタログの読み込みに失敗: {}: {}", path.display(), e))?;
    parse_catalog(&json)
}

/// spec §5.1 の不変条件。同梱ファイルの存在と寸法はカタログ自体のテスト（Task 4）で確認する
pub fn validate(entries: &[DeviceEntry]) -> Result<(), String> {
    let mut seen = HashSet::new();
    for e in entries {
        if !seen.insert(e.id.as_str()) {
            return Err(format!("カタログ id が重複しています: {}", e.id));
        }
        if e.id.is_empty()
            || !e.id.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        {
            return Err(format!("カタログ id は英小文字・数字・ハイフンのみ: {:?}", e.id));
        }
        if e.screen.right() > e.frame.width || e.screen.bottom() > e.frame.height {
            return Err(format!("{}: screen が frame の外に出ています", e.id));
        }
        if let Source::Import { pattern, .. } = &e.source {
            if pattern.matches("{variant}").count() != 1 {
                return Err(format!(
                    "{}: pattern は {{variant}} を 1 回だけ含む必要があります",
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
        assert!(validate(&e).unwrap_err().contains("重複"));
    }

    #[test]
    fn rejects_bad_id_chars() {
        let mut e = parse_catalog(SAMPLE).unwrap();
        e[0].id = "Pixel 9".into();
        assert!(validate(&e).unwrap_err().contains("英小文字"));
    }

    #[test]
    fn rejects_screen_outside_frame() {
        let mut e = parse_catalog(SAMPLE).unwrap();
        e[0].screen.x = 200; // 200 + 1080 > 1198
        assert!(validate(&e).unwrap_err().contains("frame の外"));
    }

    #[test]
    fn rejects_pattern_without_variant() {
        let mut e = parse_catalog(SAMPLE).unwrap();
        e[1].source = Source::Import { url: "u".into(), pattern: "PNG/x.png".into() };
        assert!(validate(&e).unwrap_err().contains("{variant}"));
    }

    #[test]
    fn reports_invalid_json() {
        assert!(parse_catalog("[{").unwrap_err().starts_with("カタログの読み込みに失敗"));
    }
}
