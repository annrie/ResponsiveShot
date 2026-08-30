//! フレーム画像の保存場所の解決と、UI 向けの状態（同梱 / 取り込み済み / 未取り込み）。

use std::path::PathBuf;

use serde::Serialize;

use super::catalog::{DeviceEntry, Source};

/// フレーム画像を探す 2 つのルート
#[derive(Debug, Clone)]
pub struct Roots {
    /// 同梱: `<resource_dir>/frames`
    pub bundled: PathBuf,
    /// 取り込み: `<app_data_dir>/frames`
    pub user: PathBuf,
}

/// "Black Titanium" → "black-titanium"。UI・ファイル名ともこの表記に統一する
pub fn slugify(name: &str) -> String {
    name.split_whitespace()
        .map(|s| s.to_lowercase())
        .collect::<Vec<_>>()
        .join("-")
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FrameStatus {
    pub id: String,
    pub vendor: String,
    pub category: String,
    pub name: String,
    pub orientation: String,
    /// "bundled" | "imported" | "missing"
    pub state: String,
    /// 取り込み済みの色スラッグ（昇順）。同梱は空
    pub variants: Vec<String>,
    pub source_url: Option<String>,
}

/// `<user>/<id>/*.png` のファイル名（拡張子なし）を昇順で返す
pub fn user_variants(roots: &Roots, id: &str) -> Vec<String> {
    let dir = roots.user.join(id);
    let mut names: Vec<String> = match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.extension().map_or(false, |x| x.eq_ignore_ascii_case("png")))
            .filter_map(|p| p.file_stem().map(|s| s.to_string_lossy().into_owned()))
            .collect(),
        Err(_) => Vec::new(),
    };
    names.sort();
    names
}

pub fn status_for(entry: &DeviceEntry, roots: &Roots) -> FrameStatus {
    let (state, variants, source_url) = match &entry.source {
        Source::Bundled { file } => {
            let state = if roots.bundled.join(file).is_file() { "bundled" } else { "missing" };
            (state, Vec::new(), None)
        }
        Source::Import { url, .. } => {
            let variants = user_variants(roots, &entry.id);
            let state = if variants.is_empty() { "missing" } else { "imported" };
            (state, variants, Some(url.clone()))
        }
    };
    FrameStatus {
        id: entry.id.clone(),
        vendor: entry.vendor.clone(),
        category: entry.category.clone(),
        name: entry.name.clone(),
        orientation: entry.orientation.clone(),
        state: state.to_string(),
        variants,
        source_url,
    }
}

/// 撮影に使うフレーム PNG のパス。無ければ spec §11 のメッセージでエラー
pub fn resolve_frame_png(
    entry: &DeviceEntry,
    variant: Option<&str>,
    roots: &Roots,
) -> Result<PathBuf, String> {
    let path = match &entry.source {
        Source::Bundled { file } => roots.bundled.join(file),
        Source::Import { .. } => {
            let v = variant.ok_or_else(|| format!("No color variant selected for {}", entry.name))?;
            roots.user.join(&entry.id).join(format!("{}.png", slugify(v)))
        }
    };
    if !path.is_file() {
        let label = variant.map(slugify).unwrap_or_else(|| "bundled".to_string());
        return Err(format!(
            "Frame not found: {} ({}). Import the frames again",
            entry.name, label
        ));
    }
    match image::image_dimensions(&path) {
        Ok((w, h)) if (w, h) == (entry.frame.width, entry.frame.height) => Ok(path),
        Ok((w, h)) => Err(format!(
            "Frame image size mismatch (expected {}x{}, got {}x{}): {}. Import the frames again",
            entry.frame.width,
            entry.frame.height,
            w,
            h,
            path.display()
        )),
        Err(e) => Err(format!(
            "Cannot read the frame image {}: {}. Import the frames again",
            path.display(),
            e
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use crate::frames::catalog::parse_catalog;

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

    /// テストごとに一意な一時ディレクトリ（tempfile crate は使わない方針）
    fn temp_root(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("rs-store-{}-{}-{}", tag, std::process::id(), nanos));
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
    fn slugify_lowercases_and_hyphenates() {
        assert_eq!(slugify("Black Titanium"), "black-titanium");
        assert_eq!(slugify("  White  "), "white");
        assert_eq!(slugify("Ultramarine"), "ultramarine");
    }

    #[test]
    fn bundled_status_depends_on_file_presence() {
        let entries = parse_catalog(SAMPLE).unwrap();
        let r = roots("bundled");
        assert_eq!(status_for(&entries[0], &r).state, "missing");
        touch(&r.bundled.join("google/pixel_9.png"), 1198, 2531);
        let s = status_for(&entries[0], &r);
        assert_eq!(s.state, "bundled");
        assert!(s.variants.is_empty());
        assert_eq!(s.source_url, None);
    }

    #[test]
    fn import_status_lists_variants_sorted() {
        let entries = parse_catalog(SAMPLE).unwrap();
        let r = roots("import");
        assert_eq!(status_for(&entries[1], &r).state, "missing");
        touch(&r.user.join("apple-iphone-16-pro/white-titanium.png"), 1350, 2760);
        touch(&r.user.join("apple-iphone-16-pro/black-titanium.png"), 1350, 2760);
        // notes.txt は拡張子フィルタで除外される想定なので、PNG である必要はない
        std::fs::write(r.user.join("apple-iphone-16-pro/notes.txt"), b"notes").unwrap();
        let s = status_for(&entries[1], &r);
        assert_eq!(s.state, "imported");
        assert_eq!(s.variants, vec!["black-titanium", "white-titanium"]);
        assert_eq!(s.source_url.as_deref(), Some("https://example.com/Bezel-iPhone-16.dmg"));
    }

    #[test]
    fn resolve_bundled_and_import_paths() {
        let entries = parse_catalog(SAMPLE).unwrap();
        let r = roots("resolve");
        touch(&r.bundled.join("google/pixel_9.png"), 1198, 2531);
        touch(&r.user.join("apple-iphone-16-pro/black-titanium.png"), 1350, 2760);

        assert_eq!(resolve_frame_png(&entries[0], None, &r).unwrap(), r.bundled.join("google/pixel_9.png"));
        assert_eq!(
            resolve_frame_png(&entries[1], Some("Black Titanium"), &r).unwrap(),
            r.user.join("apple-iphone-16-pro/black-titanium.png")
        );
        assert!(resolve_frame_png(&entries[1], None, &r).unwrap_err().contains("No color variant"));
        assert_eq!(
            resolve_frame_png(&entries[1], Some("pink"), &r).unwrap_err(),
            "Frame not found: iPhone 16 Pro (pink). Import the frames again"
        );
    }

    #[test]
    fn resolve_rejects_wrong_dimensions() {
        let entries = parse_catalog(SAMPLE).unwrap();
        let r = roots("wrong-dims");
        touch(&r.user.join("apple-iphone-16-pro/black-titanium.png"), 10, 10);
        let err = resolve_frame_png(&entries[1], Some("Black Titanium"), &r).unwrap_err();
        assert!(err.contains("Frame image size mismatch (expected 1350x2760, got 10x10)"), "{}", err);
    }
}
