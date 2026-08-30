//! スクリーンショットをフレーム PNG にはめ込む純関数。Tauri にも Chrome にも依存しない。

use image::imageops::{self, FilterType};
use image::{Rgba, RgbaImage};

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

/// ドロップシャドウのパラメータ。フレーム寸法に対する比率で決める（spec §9.1）
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShadowParams {
    pub sigma: f32,
    pub offset_y: u32,
    pub opacity: f32,
    pub padding: u32,
}

impl ShadowParams {
    pub fn for_frame(width: u32, height: u32) -> Self {
        let sigma = (width as f32 * 0.015).max(1.0);
        let offset_y = (height as f32 * 0.015).round() as u32;
        let padding = (3.0 * sigma + offset_y as f32).ceil() as u32;
        Self { sigma, offset_y, opacity: 0.35, padding }
    }
}

/// フレームより `padding` ずつ大きいキャンバスに、本体シルエット（フレームの不透明部 ∪ 画面矩形）を
/// 下方向に `offset_y` ずらして置き、ぼかして「黒 × opacity」にしたレイヤーを返す。
pub fn shadow_layer(frame: &RgbaImage, screen: Rect, p: &ShadowParams) -> RgbaImage {
    let (fw, fh) = frame.dimensions();
    let (cw, ch) = (fw + 2 * p.padding, fh + 2 * p.padding);

    // シルエット。画面部はフレームでは透明だが実機は塗り潰しなので矩形で埋める
    let mut mask = RgbaImage::new(cw, ch);
    for y in 0..fh {
        for x in 0..fw {
            let a = if screen.contains(x, y) { 255 } else { frame.get_pixel(x, y)[3] };
            mask.put_pixel(x + p.padding, y + p.padding + p.offset_y, Rgba([0, 0, 0, a]));
        }
    }

    // 1/4 に縮小してからぼかし、元の寸法に戻す（フルサイズの blur は 1470x3000 で数秒かかる）
    let (sw, sh) = ((cw / 4).max(1), (ch / 4).max(1));
    let small = imageops::resize(&mask, sw, sh, FilterType::Triangle);
    let blurred = imageops::blur(&small, (p.sigma / 4.0).max(0.5));
    let mut layer = imageops::resize(&blurred, cw, ch, FilterType::Triangle);
    for px in layer.pixels_mut() {
        *px = Rgba([0, 0, 0, (px[3] as f32 * p.opacity).round() as u8]);
    }
    layer
}

/// スクショを `screen` に cover リサイズして置き、その上にフレームを重ねる。
/// `shadow` が true ならキャンバスを `padding` 分広げ、影 → スクショ → フレーム の順に重ねる。
/// フレームの画面部分は透明である前提（Apple / Google の公式素材はどちらもそう）。
/// 角丸クリップはしない: フレーム側の角が不透明でスクショの角を覆う。
pub fn compose_frame(shot: &RgbaImage, frame: &RgbaImage, screen: Rect, shadow: bool) -> RgbaImage {
    let fitted = cover_resize(shot, screen.width, screen.height);
    let params = ShadowParams::for_frame(frame.width(), frame.height());
    let pad = if shadow { params.padding } else { 0 };

    let mut canvas = RgbaImage::new(frame.width() + 2 * pad, frame.height() + 2 * pad);
    if shadow {
        imageops::overlay(&mut canvas, &shadow_layer(frame, screen, &params), 0, 0);
    }
    imageops::overlay(&mut canvas, &fitted, (pad + screen.x) as i64, (pad + screen.y) as i64);
    imageops::overlay(&mut canvas, frame, pad as i64, pad as i64);
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
        let out = compose_frame(&shot, &frame_with_hole(), HOLE, false);
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
        let out = compose_frame(&shot, &frame_with_hole(), HOLE, false);
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
        let out = compose_frame(&shot, &frame_with_hole(), HOLE, false);
        assert_eq!(out.get_pixel(HOLE.right() - 1, HOLE.bottom() - 1).0, [0, 0, 255, 255]);
    }

    #[test]
    fn cover_resize_returns_exact_size_for_off_by_one_input() {
        // Pixel 9: 412 CSS px × DPR 2.625 = 1081.5 なので撮影結果が 1〜2px ずれる。これを吸収する
        let out = cover_resize(&solid(1081, 2423, [1, 2, 3, 255]), 1080, 2424);
        assert_eq!(out.dimensions(), (1080, 2424));
    }

    #[test]
    fn shadow_params_follow_frame_size() {
        let p = ShadowParams::for_frame(1350, 2760); // iPhone 16 Pro
        assert!((p.sigma - 20.25).abs() < 0.01);
        assert_eq!(p.offset_y, 41);
        assert_eq!(p.padding, 102); // ceil(60.75 + 41)
        assert_eq!(ShadowParams::for_frame(100, 200).padding, 8); // sigma 1.5, offset 3 → ceil(7.5)
    }

    #[test]
    fn shadow_expands_canvas_and_darkens_below_body() {
        let shot = solid(60, 140, [200, 0, 0, 255]);
        let p = ShadowParams::for_frame(100, 200);
        let out = compose_frame(&shot, &frame_with_hole(), HOLE, true);
        assert_eq!(out.dimensions(), (100 + 2 * p.padding, 200 + 2 * p.padding));

        let below = out.get_pixel(p.padding + 50, p.padding + 200 + 1);
        assert_eq!(&below.0[..3], &[0, 0, 0], "影は黒");
        assert!(below[3] > 0, "本体下端のすぐ外側に影がある (alpha={})", below[3]);
        assert!(out.get_pixel(0, 0)[3] <= 2, "四隅は透明（ガウスの裾は 3σ で 1% 未満）");
        assert_eq!(out.get_pixel(p.padding + 5, p.padding + 5).0, BEZEL, "本体はそのまま");
        assert_eq!(
            out.get_pixel(p.padding + 50, p.padding + 100).0,
            [200, 0, 0, 255],
            "画面もそのまま"
        );
    }
}
