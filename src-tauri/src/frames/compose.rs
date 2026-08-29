//! スクリーンショットをフレーム PNG にはめ込む純関数。Tauri にも Chrome にも依存しない。

use image::imageops::{self, FilterType};
use image::RgbaImage;

use super::Rect;

/// `src` を比率を保ったまま `w x h` を覆う大きさにリサイズし、中央で `w x h` に切り抜く
/// （CSS の object-fit: cover 相当）。寸法が一致していれば等倍コピー。
pub fn cover_resize(src: &RgbaImage, w: u32, h: u32) -> RgbaImage {
    if src.width() == w && src.height() == h {
        return src.clone();
    }
    let scale = (w as f64 / src.width() as f64).max(h as f64 / src.height() as f64);
    let rw = ((src.width() as f64 * scale).ceil() as u32).max(w);
    let rh = ((src.height() as f64 * scale).ceil() as u32).max(h);
    let resized = imageops::resize(src, rw, rh, FilterType::Lanczos3);
    imageops::crop_imm(&resized, (rw - w) / 2, (rh - h) / 2, w, h).to_image()
}

/// スクショを `screen` に cover リサイズして置き、その上にフレームを重ねる。
/// フレームの画面部分は透明である前提（Apple / Google の公式素材はどちらもそう）。
/// 角丸クリップはしない: フレーム側の角が不透明でスクショの角を覆う。
pub fn compose_frame(shot: &RgbaImage, frame: &RgbaImage, screen: Rect) -> RgbaImage {
    let fitted = cover_resize(shot, screen.width, screen.height);
    let mut canvas = RgbaImage::new(frame.width(), frame.height());
    imageops::overlay(&mut canvas, &fitted, screen.x as i64, screen.y as i64);
    imageops::overlay(&mut canvas, frame, 0, 0);
    canvas
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::Rgba;

    const HOLE: Rect = Rect { x: 20, y: 30, width: 60, height: 140 };
    const BEZEL: [u8; 4] = [10, 20, 30, 255];

    fn solid(w: u32, h: u32, c: [u8; 4]) -> RgbaImage {
        RgbaImage::from_pixel(w, h, Rgba(c))
    }

    /// 100x200 のフレーム。HOLE の内側だけ透明（Apple / Google の公式素材と同じ構造）
    fn frame_with_hole() -> RgbaImage {
        let mut f = solid(100, 200, BEZEL);
        for y in HOLE.y..HOLE.bottom() {
            for x in HOLE.x..HOLE.right() {
                f.put_pixel(x, y, Rgba([0, 0, 0, 0]));
            }
        }
        f
    }

    #[test]
    fn screenshot_fills_hole_and_frame_covers_outside() {
        let shot = solid(60, 140, [200, 0, 0, 255]);
        let out = compose_frame(&shot, &frame_with_hole(), HOLE);
        assert_eq!(out.dimensions(), (100, 200));
        assert_eq!(out.get_pixel(50, 100).0, [200, 0, 0, 255], "画面中央はスクショ");
        assert_eq!(out.get_pixel(5, 5).0, BEZEL, "ベゼル部分はフレーム");
        assert_eq!(out.get_pixel(HOLE.x, HOLE.y).0, [200, 0, 0, 255], "画面の左上角");
    }

    #[test]
    fn wider_screenshot_is_cover_cropped_to_center() {
        // 120x140: 高さは一致、幅が 2 倍。cover なので左右 30px ずつ切り捨てられる
        let mut shot = solid(120, 140, [0, 0, 255, 255]);
        for y in 0..140 {
            for x in 0..30 {
                shot.put_pixel(x, y, Rgba([0, 255, 0, 255]));
            }
        }
        let out = compose_frame(&shot, &frame_with_hole(), HOLE);
        assert_eq!(out.dimensions(), (100, 200));
        assert_eq!(
            out.get_pixel(HOLE.x, HOLE.y + 70).0,
            [0, 0, 255, 255],
            "左端 30px の緑は切り落とされる"
        );
    }

    #[test]
    fn smaller_screenshot_is_upscaled_to_fill() {
        let shot = solid(30, 70, [0, 0, 255, 255]);
        let out = compose_frame(&shot, &frame_with_hole(), HOLE);
        assert_eq!(out.get_pixel(HOLE.right() - 1, HOLE.bottom() - 1).0, [0, 0, 255, 255]);
    }

    #[test]
    fn cover_resize_returns_exact_size_for_off_by_one_input() {
        // Pixel 9: 412 CSS px × DPR 2.625 = 1081.5 なので撮影結果が 1〜2px ずれる。これを吸収する
        let out = cover_resize(&solid(1081, 2423, [1, 2, 3, 255]), 1080, 2424);
        assert_eq!(out.dimensions(), (1080, 2424));
    }
}
