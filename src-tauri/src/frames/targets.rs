//! `capture_screenshots` が使う `CaptureTarget` の生成ロジック。main.rs から切り出してユニットテストできるようにする。

use std::path::PathBuf;

use image::Rgba;
use serde::Deserialize;

use super::catalog::{self, DeviceEntry};
use super::store::{self, Roots};
use super::Rect;

#[derive(Debug, Deserialize)]
pub struct DeviceSelection {
    pub id: String,
    pub variant: Option<String>,
}

/// フレーム合成の指示。Some のターゲットは viewport 固定・PNG 固定で、保存前に合成する
#[derive(Debug)]
pub struct FrameJob {
    pub frame_png: PathBuf,
    pub screen: Rect,
    pub shadow: bool,
    /// フレーム外の背景色。None = 透明
    pub background: Option<Rgba<u8>>,
}

/// 1 回のブラウザ起動で撮る対象。幅指定は dpr 1.0 / mobile false、デバイスはカタログの値
#[derive(Debug)]
pub struct CaptureTarget {
    pub width: u32,
    pub height: u32,
    pub dpr: f64,
    pub mobile: bool,
    /// ファイル名用ラベル。幅指定は従来どおり "1440px" / "1440x810"
    pub label: String,
    pub frame: Option<FrameJob>,
}

/// 幅ターゲット（従来どおり dpr 1.0 / mobile false）に続けてデバイスターゲットを並べる。
/// `frames` はデバイスが 1 件以上あるときだけ必要（カタログとルート）。
pub fn build_targets(
    widths: &[u32],
    viewport_height: Option<u32>,
    capture_height: u32,
    devices: &[DeviceSelection],
    frame_shadow: bool,
    frame_background: Option<Rgba<u8>>,
    duration: u32,
    frames: Option<(&[DeviceEntry], &Roots)>,
) -> Result<Vec<CaptureTarget>, String> {
    let mut targets: Vec<CaptureTarget> = widths
        .iter()
        .map(|&w| CaptureTarget {
            width: w,
            height: capture_height,
            dpr: 1.0,
            mobile: false,
            label: if viewport_height.is_some() {
                format!("{}x{}", w, capture_height)
            } else {
                format!("{}px", w)
            },
            frame: None,
        })
        .collect();

    if !devices.is_empty() {
        if duration > 0 {
            return Err("デバイスフレームは PNG 出力のみ対応しています".to_string());
        }
        let (entries, roots) =
            frames.ok_or_else(|| "フレームカタログが読み込まれていません".to_string())?;
        for sel in devices {
            let entry = catalog::find(entries, &sel.id)
                .ok_or_else(|| format!("カタログに無いデバイスです: {}", sel.id))?;
            // フレームが無ければここで止める（撮影を始めない）
            let frame_png = store::resolve_frame_png(entry, sel.variant.as_deref(), roots)?;
            let label = match &sel.variant {
                Some(v) => format!("{}_{}", entry.id, store::slugify(v)),
                None => entry.id.clone(),
            };
            targets.push(CaptureTarget {
                width: entry.css.width,
                height: entry.css.height,
                dpr: entry.css.dpr,
                mobile: entry.css.mobile,
                label,
                frame: Some(FrameJob {
                    frame_png,
                    screen: entry.screen,
                    shadow: frame_shadow,
                    background: frame_background,
                }),
            });
        }
    }

    Ok(targets)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

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

    fn temp_root(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("rs-targets-{}-{}-{}", tag, std::process::id(), nanos));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// 指定サイズの実 PNG を書き出す（`resolve_frame_png` の寸法検証を通すため）
    fn touch(path: &Path, w: u32, h: u32) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        image::RgbaImage::new(w, h).save(path).unwrap();
    }

    fn roots(tag: &str) -> Roots {
        let base = temp_root(tag);
        Roots { bundled: base.join("bundled"), user: base.join("user") }
    }

    #[test]
    fn width_targets_match_legacy_labels() {
        let widths = [375, 1440];

        let targets = build_targets(&widths, None, 1080, &[], false, None, 0, None).unwrap();
        assert_eq!(targets.len(), 2);
        assert_eq!(targets[0].label, "375px");
        assert_eq!(targets[1].label, "1440px");
        for t in &targets {
            assert_eq!(t.dpr, 1.0);
            assert!(!t.mobile);
            assert!(t.frame.is_none());
        }

        let targets = build_targets(&widths, Some(810), 810, &[], false, None, 0, None).unwrap();
        assert_eq!(targets[0].label, "375x810");
        assert_eq!(targets[1].label, "1440x810");
    }

    #[test]
    fn gif_with_devices_is_rejected_before_anything_else() {
        let devices = [DeviceSelection { id: "google-pixel-9".into(), variant: None }];
        let err = build_targets(&[], None, 1080, &devices, false, None, 3, None).unwrap_err();
        assert_eq!(err, "デバイスフレームは PNG 出力のみ対応しています");
    }

    #[test]
    fn missing_frame_fails_with_store_message() {
        let entries = catalog::parse_catalog(SAMPLE).unwrap();
        let r = roots("missing");
        let devices = [DeviceSelection { id: "google-pixel-9".into(), variant: None }];
        let err = build_targets(&[], None, 1080, &devices, false, None, 0, Some((&entries, &r))).unwrap_err();
        assert!(err.contains("フレームが見つかりません"), "{}", err);
    }

    #[test]
    fn bundled_device_target_uses_catalog_css_and_screen() {
        let entries = catalog::parse_catalog(SAMPLE).unwrap();
        let r = roots("bundled");
        touch(&r.bundled.join("google/pixel_9.png"), 1198, 2531);
        let devices = [DeviceSelection { id: "google-pixel-9".into(), variant: None }];
        let targets = build_targets(&[], None, 1080, &devices, true, None, 0, Some((&entries, &r))).unwrap();
        assert_eq!(targets.len(), 1);
        let t = &targets[0];
        assert_eq!(t.width, 412);
        assert_eq!(t.height, 923);
        assert_eq!(t.dpr, 2.625);
        assert!(t.mobile);
        assert_eq!(t.label, "google-pixel-9");
        let job = t.frame.as_ref().unwrap();
        assert_eq!(job.screen, entries[0].screen);
        assert_eq!(job.shadow, true);
    }

    #[test]
    fn device_target_carries_background() {
        let entries = catalog::parse_catalog(SAMPLE).unwrap();
        let r = roots("bg");
        touch(&r.bundled.join("google/pixel_9.png"), 1198, 2531);
        let devices = [DeviceSelection { id: "google-pixel-9".into(), variant: None }];
        let white = image::Rgba([255, 255, 255, 255]);
        let targets = build_targets(&[], None, 1080, &devices, false, Some(white), 0, Some((&entries, &r))).unwrap();
        assert_eq!(targets[0].frame.as_ref().unwrap().background, Some(white));
        let targets = build_targets(&[], None, 1080, &devices, false, None, 0, Some((&entries, &r))).unwrap();
        assert_eq!(targets[0].frame.as_ref().unwrap().background, None);
    }

    #[test]
    fn import_device_label_includes_variant_slug() {
        let entries = catalog::parse_catalog(SAMPLE).unwrap();
        let r = roots("import");
        touch(&r.user.join("apple-iphone-16-pro/black-titanium.png"), 1350, 2760);
        let devices = [DeviceSelection {
            id: "apple-iphone-16-pro".into(),
            variant: Some("Black Titanium".into()),
        }];
        let targets = build_targets(&[], None, 1080, &devices, false, None, 0, Some((&entries, &r))).unwrap();
        assert_eq!(targets[0].label, "apple-iphone-16-pro_black-titanium");
        assert_eq!(targets[0].dpr, 3.0);
    }

    #[test]
    fn unknown_device_id_is_rejected() {
        let entries = catalog::parse_catalog(SAMPLE).unwrap();
        let r = roots("unknown");
        let devices = [DeviceSelection { id: "nope".into(), variant: None }];
        let err = build_targets(&[], None, 1080, &devices, false, None, 0, Some((&entries, &r))).unwrap_err();
        assert!(err.contains("カタログに無いデバイスです"), "{}", err);
    }
}
