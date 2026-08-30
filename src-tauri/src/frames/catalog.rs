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

/// Dynamic Island の黒塗り領域（CSS px、画面左上原点）。radius は角丸半径（高さ ÷ 2 でピル形）
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Island {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub radius: f64,
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
    /// iPhone 16 系だけが持つ。無い機種は黒塗りしない
    #[serde(default)]
    pub island: Option<Island>,
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
        if let Some(i) = e.island {
            let fits = i.width > 0.0
                && i.height > 0.0
                && i.x >= 0.0
                && i.y >= 0.0
                && i.x + i.width <= e.css.width as f64
                && i.y + i.height <= e.css.height as f64
                && i.radius >= 0.0
                // カタログ値は小数第2位で個別に丸められているため、height / 2 との差が丸め誤差の範囲(最大 0.01)に収まっていれば許容する
                && i.radius <= i.height / 2.0 + 0.01;
            if !fits {
                return Err(format!("{}: island must fit inside the css viewport", e.id));
            }
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

    #[test]
    fn island_is_optional_and_deserializes() {
        let json = r#"[{
          "id": "apple-iphone-16", "vendor": "apple", "category": "phone", "name": "iPhone 16",
          "orientation": "portrait",
          "css": { "width": 393, "height": 852, "dpr": 3, "mobile": true },
          "frame": { "width": 1359, "height": 2736 },
          "screen": { "x": 90, "y": 90, "width": 1179, "height": 2556 },
          "source": { "kind": "import", "url": "https://example.com/x.dmg", "pattern": "PNG/{variant}.png" },
          "island": { "x": 133.5, "y": 11, "width": 126, "height": 37.33, "radius": 18.67 }
        }, {
          "id": "google-pixel-9", "vendor": "google", "category": "phone", "name": "Pixel 9",
          "orientation": "portrait",
          "css": { "width": 412, "height": 923, "dpr": 2.625, "mobile": true },
          "frame": { "width": 1200, "height": 2500 },
          "screen": { "x": 60, "y": 60, "width": 1080, "height": 2400 },
          "source": { "kind": "bundled", "file": "google/pixel-9.png" }
        }]"#;
        let entries = parse_catalog(json).unwrap();
        let island = entries[0].island.expect("iPhone 16 has an island");
        assert_eq!(island, Island { x: 133.5, y: 11.0, width: 126.0, height: 37.33, radius: 18.67 });
        assert_eq!(entries[1].island, None, "island を省略した機種は None");
    }

    #[test]
    fn rejects_island_outside_css_viewport() {
        let mut e = parse_catalog(SAMPLE).unwrap(); // e[1] は iPhone 16 Pro(css.width 402)
        e[1].island = Some(Island { x: 300.0, y: 11.0, width: 126.0, height: 37.33, radius: 18.67 }); // 300 + 126 > 402
        assert!(validate(&e).unwrap_err().contains("island must fit inside the css viewport"));
        e[1].island = Some(Island { x: 138.0, y: 11.0, width: 126.0, height: 37.33, radius: 30.0 }); // radius > height / 2
        assert!(validate(&e).unwrap_err().contains("island must fit inside the css viewport"));
        e[1].island = Some(Island { x: 138.0, y: 11.0, width: 126.0, height: 37.33, radius: 18.67 });
        assert!(validate(&e).is_ok(), "収まっていれば合格");
    }

    #[test]
    fn bundled_catalog_has_islands_only_on_iphone_16_family() {
        let entries = load_catalog(Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/frames/catalog.json"))).unwrap();
        let with: Vec<&str> = entries.iter().filter(|e| e.island.is_some()).map(|e| e.id.as_str()).collect();
        assert_eq!(with, ["apple-iphone-16", "apple-iphone-16-plus", "apple-iphone-16-pro", "apple-iphone-16-pro-max"]);

        // ベゼル PNG の穴内側にある非透明画素の外接矩形から実測した値（2026-08-31）
        let expected = [
            ("apple-iphone-16", Island { x: 134.0, y: 11.0, width: 125.0, height: 37.33, radius: 18.67 }),
            ("apple-iphone-16-plus", Island { x: 152.33, y: 11.33, width: 125.67, height: 36.67, radius: 18.33 }),
            ("apple-iphone-16-pro", Island { x: 138.67, y: 14.33, width: 124.67, height: 36.0, radius: 18.0 }),
            ("apple-iphone-16-pro-max", Island { x: 157.67, y: 14.33, width: 124.67, height: 36.0, radius: 18.0 }),
        ];
        for (id, island) in expected {
            assert_eq!(find(&entries, id).unwrap().island, Some(island), "{}", id);
        }
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
}
