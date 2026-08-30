//! Apple 公式 DMG / フォルダ / PNG からフレーム画像を取り込む。

use std::path::{Path, PathBuf};

use serde::Serialize;

use super::catalog::{DeviceEntry, Source};
use super::store::slugify;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::frames::catalog::{CssSpec, Size};
    use crate::frames::Rect;
    use image::RgbaImage;

    fn entry(id: &str, name: &str, pattern: &str, w: u32, h: u32) -> DeviceEntry {
        DeviceEntry {
            id: id.into(),
            vendor: "apple".into(),
            category: "phone".into(),
            name: name.into(),
            orientation: "portrait".into(),
            css: CssSpec { width: 402, height: 874, dpr: 3.0, mobile: true },
            frame: Size { width: w, height: h },
            screen: Rect { x: 1, y: 1, width: w - 2, height: h - 2 },
            source: Source::Import { url: "https://example.com/x.dmg".into(), pattern: pattern.into() },
        }
    }

    fn pro() -> DeviceEntry {
        entry("apple-iphone-16-pro", "iPhone 16 Pro", "PNG/iPhone 16 Pro/iPhone 16 Pro - {variant} - Portrait.png", 100, 200)
    }

    fn temp_root(tag: &str) -> PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("rs-import-{}-{}-{}", tag, std::process::id(), nanos));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn write_png(path: &Path, w: u32, h: u32) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        RgbaImage::new(w, h).save(path).unwrap();
    }

    #[test]
    fn split_pattern_splits_at_single_placeholder() {
        let p = split_pattern("PNG/iPhone 16 Pro/iPhone 16 Pro - {variant} - Portrait.png").unwrap();
        assert_eq!(p.prefix, "PNG/iPhone 16 Pro/iPhone 16 Pro - ");
        assert_eq!(p.suffix, " - Portrait.png");
        assert!(split_pattern("PNG/x.png").is_none());
        assert!(split_pattern("{variant}/{variant}.png").is_none());
    }

    #[test]
    fn match_variant_by_path_and_by_file_name() {
        let p = split_pattern("PNG/iPhone 16 Pro/iPhone 16 Pro - {variant} - Portrait.png").unwrap();
        assert_eq!(
            match_variant(&p, "PNG/iPhone 16 Pro/iPhone 16 Pro - Black Titanium - Portrait.png", false),
            Some("Black Titanium".to_string())
        );
        assert_eq!(match_variant(&p, "iPhone 16 Pro - Black Titanium - Portrait.png", true), Some("Black Titanium".to_string()));
        assert_eq!(match_variant(&p, "PNG/iPhone 16 Pro Max/iPhone 16 Pro Max - Black Titanium - Portrait.png", false), None);
        assert_eq!(match_variant(&p, "iPhone 16 Pro Max - Black Titanium - Portrait.png", true), None);
        assert_eq!(match_variant(&p, "iPhone 16 Pro - Black Titanium - Landscape.png", true), None);
        assert_eq!(match_variant(&p, "iPhone 16 Pro -  - Portrait.png", true), None, "色名が空");
    }

    #[test]
    fn import_copies_matching_png_as_slug() {
        let root = temp_root("copy");
        let user = root.join("user");
        write_png(&root.join("PNG/iPhone 16 Pro/iPhone 16 Pro - Black Titanium - Portrait.png"), 100, 200);
        write_png(&root.join("PNG/iPhone 16 Pro Max/iPhone 16 Pro Max - Black Titanium - Portrait.png"), 110, 220);

        let files = scan_pngs(&root);
        let report = import_pngs(&files, &root, false, &[pro()], &user).unwrap();

        assert_eq!(report.imported, vec![ImportedFrame { id: "apple-iphone-16-pro".into(), variant: "black-titanium".into() }]);
        assert!(report.skipped.is_empty(), "Pro Max はどのエントリにも合わないので黙って無視");
        assert!(user.join("apple-iphone-16-pro/black-titanium.png").is_file());
    }

    #[test]
    fn import_skips_dimension_mismatch_and_hidden_files() {
        let root = temp_root("skip");
        let user = root.join("user");
        write_png(&root.join("PNG/iPhone 16 Pro/iPhone 16 Pro - White Titanium - Portrait.png"), 90, 200);
        write_png(&root.join("PNG/iPhone 16 Pro/._iPhone 16 Pro - Black Titanium - Portrait.png"), 100, 200);

        let files = scan_pngs(&root);
        assert_eq!(files.len(), 1, "._ ファイルは走査対象外");
        let report = import_pngs(&files, &root, false, &[pro()], &user).unwrap();

        assert!(report.imported.is_empty());
        assert_eq!(report.skipped.len(), 1);
        assert_eq!(report.skipped[0].reason, "寸法が不一致 (期待 100x200, 実際 90x200)");
        assert!(!user.join("apple-iphone-16-pro").exists());
    }

    #[test]
    fn import_by_file_name_from_flat_folder() {
        let root = temp_root("flat");
        let user = root.join("user");
        write_png(&root.join("iPhone 16 Pro - Natural Titanium - Portrait.png"), 100, 200);
        let files = scan_pngs(&root);
        let report = import_pngs(&files, &root, true, &[pro()], &user).unwrap();
        assert_eq!(report.imported[0].variant, "natural-titanium");
    }

    #[test]
    fn import_frames_dispatches_on_folder_file_and_missing() {
        let root = temp_root("dispatch");
        let user = root.join("user");
        let png = root.join("iPhone 16 Pro - Desert Titanium - Portrait.png");
        write_png(&png, 100, 200);

        let by_dir = import_frames(&root, &[pro()], &user).unwrap();
        assert_eq!(by_dir.imported[0].variant, "desert-titanium");

        let by_file = import_frames(&png, &[pro()], &user).unwrap();
        assert_eq!(by_file.imported.len(), 1);

        let err = import_frames(&root.join("nope.dmg"), &[pro()], &user).unwrap_err();
        assert!(
            err.contains("DMG のマウントに失敗") || err.contains("hdiutil を起動できません"),
            "{}",
            err
        );
    }
}

/// `pattern` を `{variant}` の前後で分けたもの
#[derive(Debug, Clone, PartialEq)]
pub struct PatternParts {
    pub prefix: String,
    pub suffix: String,
}

/// `{variant}` をちょうど 1 回含むときだけ Some
pub fn split_pattern(pattern: &str) -> Option<PatternParts> {
    let mut it = pattern.splitn(2, "{variant}");
    let prefix = it.next()?.to_string();
    let suffix = it.next()?.to_string();
    if suffix.contains("{variant}") {
        return None;
    }
    Some(PatternParts { prefix, suffix })
}

/// DMG: ボリューム相対パス全体で照合。フォルダ / 単一 PNG: ファイル名を prefix の最後のパス成分以降と照合。
/// 間の文字列（色名）が空、または `/` を含むものは不一致
pub fn match_variant(parts: &PatternParts, candidate: &str, by_file_name: bool) -> Option<String> {
    let prefix: &str = if by_file_name {
        parts.prefix.rsplit('/').next().unwrap_or(&parts.prefix)
    } else {
        &parts.prefix
    };
    let rest = candidate.strip_prefix(prefix)?;
    let middle = rest.strip_suffix(parts.suffix.as_str())?;
    if middle.is_empty() || middle.contains('/') {
        return None;
    }
    Some(middle.to_string())
}

/// `root` 以下の PNG を再帰的に集める（`.` で始まる名前 = macOS の `._` リソースフォークや `.fseventsd` は除外）
pub fn scan_pngs(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let rd = match std::fs::read_dir(&dir) {
            Ok(rd) => rd,
            Err(_) => continue,
        };
        for entry in rd.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().map_or(false, |x| x.eq_ignore_ascii_case("png")) {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ImportedFrame {
    pub id: String,
    pub variant: String,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct SkippedFile {
    pub file: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize)]
pub struct ImportReport {
    pub imported: Vec<ImportedFrame>,
    pub skipped: Vec<SkippedFile>,
}

/// `files` をカタログの import エントリに照合し、寸法が合うものを `<user_dir>/<id>/<variant-slug>.png` にコピーする。
/// どのエントリにも合わないファイルは黙って無視する（DMG には PSD 等も入っている）。
pub fn import_pngs(
    files: &[PathBuf],
    root: &Path,
    by_file_name: bool,
    entries: &[DeviceEntry],
    user_dir: &Path,
) -> Result<ImportReport, String> {
    let patterns: Vec<(&DeviceEntry, PatternParts)> = entries
        .iter()
        .filter_map(|e| match &e.source {
            Source::Import { pattern, .. } => split_pattern(pattern).map(|p| (e, p)),
            Source::Bundled { .. } => None,
        })
        .collect();

    let mut report = ImportReport::default();
    for file in files {
        let candidate = if by_file_name {
            file.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
        } else {
            file.strip_prefix(root)
                .map_err(|e| e.to_string())?
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/")
        };

        let matched = patterns
            .iter()
            .find_map(|(e, p)| match_variant(p, &candidate, by_file_name).map(|v| (*e, v)));
        let (entry, variant) = match matched {
            Some(m) => m,
            None => continue,
        };

        let (w, h) = match image::image_dimensions(file) {
            Ok(d) => d,
            Err(e) => {
                report.skipped.push(SkippedFile { file: candidate, reason: format!("画像として読めません: {}", e) });
                continue;
            }
        };
        if (w, h) != (entry.frame.width, entry.frame.height) {
            report.skipped.push(SkippedFile {
                file: candidate,
                reason: format!(
                    "寸法が不一致 (期待 {}x{}, 実際 {}x{})",
                    entry.frame.width, entry.frame.height, w, h
                ),
            });
            continue;
        }

        let slug = slugify(&variant);
        let dest_dir = user_dir.join(&entry.id);
        std::fs::create_dir_all(&dest_dir)
            .map_err(|e| format!("保存先を作成できません {}: {}", dest_dir.display(), e))?;
        let dest = dest_dir.join(format!("{}.png", slug));
        std::fs::copy(file, &dest)
            .map_err(|e| format!("コピーに失敗 {} → {}: {}", file.display(), dest.display(), e))?;
        report.imported.push(ImportedFrame { id: entry.id.clone(), variant: slug });
    }
    Ok(report)
}

/// `hdiutil attach` したボリューム。Drop で必ず detach する（エラー経路含む）
pub struct DmgMount {
    mountpoint: PathBuf,
}

impl DmgMount {
    pub fn attach(dmg: &Path) -> Result<Self, String> {
        use std::io::Write;
        use std::process::{Command, Stdio};

        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0);
        let mountpoint =
            std::env::temp_dir().join(format!("responsiveshot-dmg-{}-{}", std::process::id(), nanos));
        std::fs::create_dir_all(&mountpoint).map_err(|e| e.to_string())?;

        let mut child = Command::new("hdiutil")
            .args(["attach", "-nobrowse", "-readonly", "-mountpoint"])
            .arg(&mountpoint)
            .arg(dmg)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| format!("hdiutil を起動できません: {}", e))?;
        if let Some(mut stdin) = child.stdin.take() {
            // Apple の DMG は使用許諾への同意を求める。非対話なので stdin で Y を返す
            let _ = stdin.write_all(b"Y\nY\nY\nY\n");
        }
        let output = child.wait_with_output().map_err(|e| e.to_string())?;
        if !output.status.success() {
            let _ = std::fs::remove_dir(&mountpoint);
            return Err(format!(
                "DMG のマウントに失敗: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            ));
        }
        Ok(Self { mountpoint })
    }

    pub fn path(&self) -> &Path {
        &self.mountpoint
    }
}

impl Drop for DmgMount {
    fn drop(&mut self) {
        let _ = std::process::Command::new("hdiutil")
            .args(["detach", "-quiet"])
            .arg(&self.mountpoint)
            .output();
        let _ = std::fs::remove_dir(&self.mountpoint);
    }
}

/// 取り込みの入口。`.dmg` / フォルダ / 単一 PNG のいずれかを受け付ける
pub fn import_frames(path: &Path, entries: &[DeviceEntry], user_dir: &Path) -> Result<ImportReport, String> {
    let is_dmg = path.extension().map_or(false, |x| x.eq_ignore_ascii_case("dmg"));
    if is_dmg {
        let mount = DmgMount::attach(path)?;
        let files = scan_pngs(mount.path());
        import_pngs(&files, mount.path(), false, entries, user_dir)
    } else if path.is_dir() {
        let files = scan_pngs(path);
        import_pngs(&files, path, true, entries, user_dir)
    } else if path.is_file() {
        let root = path.parent().unwrap_or(path);
        import_pngs(&[path.to_path_buf()], root, true, entries, user_dir)
    } else {
        Err(format!("取り込み元が見つかりません: {}", path.display()))
    }
}
