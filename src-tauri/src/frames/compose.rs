//! スクリーンショットをフレーム PNG にはめ込む純関数。Tauri にも Chrome にも依存しない。

use image::imageops::{self, FilterType};
use image::{Rgba, RgbaImage};

use super::Rect;

/// `#rgb` / `#rrggbb` を不透明色に変換する。前後の空白は無視。それ以外は Err
pub fn parse_hex_color(s: &str) -> Result<Rgba<u8>, String> {
    let err = || format!("Invalid background color: {:?} (use #rrggbb)", s);
    let t = s.trim();
    let hex = t.strip_prefix('#').ok_or_else(err)?;
    if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(err());
    }
    let expanded: String = match hex.len() {
        3 => hex.chars().flat_map(|c| [c, c]).collect(),
        6 => hex.to_string(),
        _ => return Err(err()),
    };
    let channel = |i: usize| u8::from_str_radix(&expanded[i..i + 2], 16).map_err(|_| err());
    Ok(Rgba([channel(0)?, channel(2)?, channel(4)?, 255]))
}

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

/// フレームより `padding` ずつ大きいキャンバスに、本体シルエット（フレームの不透明部 ∪ 画面の穴マスク）を
/// 下方向に `offset_y` ずらして置き、ぼかして「黒 × opacity」にしたレイヤーを返す。
/// `mask` は `screen_mask(frame, screen)` の戻り値（screen 矩形と同じ大きさ・行優先の bool 配列）を渡す。
/// Apple のベゼルのように screen 矩形の角が本体の外（透明）に出るフレームでは、画面部を矩形のまま
/// 埋めるとシルエットの角が四角いままになり、丸い角の外に灰色の影がはみ出す。穴マスクで絞ることで
/// スクショのクリップと同じ輪郭になる。
pub fn shadow_layer(frame: &RgbaImage, screen: Rect, mask: &[bool], p: &ShadowParams) -> RgbaImage {
    let (fw, fh) = frame.dimensions();
    let (cw, ch) = (fw + 2 * p.padding, fh + 2 * p.padding);

    // シルエット。画面部はフレームでは透明だが実機は塗り潰しなので穴マスクで埋める
    let mut silhouette = RgbaImage::new(cw, ch);
    for y in 0..fh {
        for x in 0..fw {
            let in_hole = screen.contains(x, y)
                && mask[(y - screen.y) as usize * screen.width as usize + (x - screen.x) as usize];
            let a = if in_hole { 255 } else { frame.get_pixel(x, y)[3] };
            silhouette.put_pixel(x + p.padding, y + p.padding + p.offset_y, Rgba([0, 0, 0, a]));
        }
    }

    // 1/4 に縮小してからぼかし、元の寸法に戻す（フルサイズの blur は 1470x3000 で数秒かかる）
    let (sw, sh) = ((cw / 4).max(1), (ch / 4).max(1));
    let small = imageops::resize(&silhouette, sw, sh, FilterType::Triangle);
    let blurred = imageops::blur(&small, (p.sigma / 4.0).max(0.5));
    let mut layer = imageops::resize(&blurred, cw, ch, FilterType::Triangle);
    for px in layer.pixels_mut() {
        *px = Rgba([0, 0, 0, (px[3] as f32 * p.opacity).round() as u8]);
    }
    layer
}

/// フレームの「穴」（画面中央付近から連結している非不透明画素）を screen 矩形内でフラッドフィルして求める。
/// フラッドフィルの起点は中央画素とは限らない。中央が不透明（フレームの装飾等と重なっている）な場合は
/// 中央を囲む正方形リングをチェビシェフ距離 1, 2, … の順に走査し、最初に見つかった非不透明画素を起点にする。
/// 戻り値は screen 矩形と同じ大きさの bool 配列（行優先）。screen 矩形全体が不透明なら全画素 true（クリップしない）。
/// Apple のベゼルは角の丸みが大きく、画面矩形の角が本体の外（透明）に出るため、矩形のまま置くと
/// スクショの角がはみ出す。穴の形でクリップすればフレームの種類に依らず正しく収まる。
pub fn screen_mask(frame: &RgbaImage, screen: Rect) -> Vec<bool> {
    let (w, h) = (screen.width as usize, screen.height as usize);
    let mut mask = vec![false; w * h];
    if w == 0 || h == 0 {
        return mask;
    }
    let passable = |x: usize, y: usize| -> bool {
        let px = screen.x + x as u32;
        let py = screen.y + y as u32;
        px < frame.width() && py < frame.height() && frame.get_pixel(px, py)[3] < 250
    };
    let (center_x, center_y) = (w / 2, h / 2);
    // 起点探索: 中央が通行可能ならそこで即決（よくあるケースは O(1)）。そうでなければ中央を囲む
    // 正方形リングをチェビシェフ距離 r = 1, 2, … の順に外側へ広げ、周だけを走査して最初に見つかった
    // 通行可能画素を起点にする。矩形全体（探索半径 = max(w, h) まで）が不透明ならクリップなしで返す。
    let (cxi, cyi) = (center_x as i64, center_y as i64);
    let max_r = w.max(h) as i64;
    let mut seed = None;
    'search: for r in 0..=max_r {
        if r == 0 {
            if passable(center_x, center_y) {
                seed = Some((center_x, center_y));
                break 'search;
            }
            continue;
        }
        let (lo, hi) = (-r, r);
        for dy in [lo, hi] {
            let py = cyi + dy;
            if py < 0 || py as usize >= h {
                continue;
            }
            for dx in lo..=hi {
                let px = cxi + dx;
                if px < 0 || px as usize >= w {
                    continue;
                }
                let (x, y) = (px as usize, py as usize);
                if passable(x, y) {
                    seed = Some((x, y));
                    break 'search;
                }
            }
        }
        for dx in [lo, hi] {
            let px = cxi + dx;
            if px < 0 || px as usize >= w {
                continue;
            }
            for dy in (lo + 1)..hi {
                let py = cyi + dy;
                if py < 0 || py as usize >= h {
                    continue;
                }
                let (x, y) = (px as usize, py as usize);
                if passable(x, y) {
                    seed = Some((x, y));
                    break 'search;
                }
            }
        }
    }
    let (cx, cy) = match seed {
        Some(p) => p,
        None => return vec![true; w * h],
    };
    let mut queue = std::collections::VecDeque::with_capacity(w.max(h) * 4);
    mask[cy * w + cx] = true;
    queue.push_back((cx, cy));
    while let Some((x, y)) = queue.pop_front() {
        let neighbors = [
            (x.wrapping_sub(1), y),
            (x + 1, y),
            (x, y.wrapping_sub(1)),
            (x, y + 1),
        ];
        for (nx, ny) in neighbors {
            if nx < w && ny < h && !mask[ny * w + nx] && passable(nx, ny) {
                mask[ny * w + nx] = true;
                queue.push_back((nx, ny));
            }
        }
    }
    mask
}

/// スクショを `screen` に cover リサイズして置き、その上にフレームを重ねる。
/// `shadow` が true ならキャンバスを `padding` 分広げ、影 → スクショ → フレーム の順に重ねる。
/// `background` を指定するとキャンバスをその色（不透明）で初期化する。`None` は透明（従来どおり）。
/// フレームの画面部分は透明である前提（Apple / Google の公式素材はどちらもそう）。
/// スクショはフレームの穴（`screen_mask`）でクリップするので、丸い角や本体の外にはみ出さない。
pub fn compose_frame(
    shot: &RgbaImage,
    frame: &RgbaImage,
    screen: Rect,
    shadow: bool,
    background: Option<Rgba<u8>>,
) -> RgbaImage {
    let fitted = cover_resize(shot, screen.width, screen.height);
    // 穴の外に出るスクショ画素は透明にする（Apple ベゼルの丸い角対策）
    let mask = screen_mask(frame, screen);
    let mut fitted = fitted;
    for (i, px) in fitted.pixels_mut().enumerate() {
        if !mask[i] {
            *px = Rgba([0, 0, 0, 0]);
        }
    }
    let params = ShadowParams::for_frame(frame.width(), frame.height());
    let pad = if shadow { params.padding } else { 0 };

    let (cw, ch) = (frame.width() + 2 * pad, frame.height() + 2 * pad);
    let mut canvas = match background {
        Some(color) => RgbaImage::from_pixel(cw, ch, color),
        None => RgbaImage::new(cw, ch),
    };
    if shadow {
        imageops::overlay(&mut canvas, &shadow_layer(frame, screen, &mask, &params), 0, 0);
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
        let out = compose_frame(&shot, &frame_with_hole(), HOLE, false, None);
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
        let out = compose_frame(&shot, &frame_with_hole(), HOLE, false, None);
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
        let out = compose_frame(&shot, &frame_with_hole(), HOLE, false, None);
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
        let out = compose_frame(&shot, &frame_with_hole(), HOLE, true, None);
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

    #[test]
    fn shadow_silhouette_follows_the_hole_mask() {
        // 丸角リング（frame_with_rounded_ring）は screen 矩形の角が本体の外（透明）に出る。
        // シルエットが screen.contains だけで矩形に埋めていると角も含めて不透明になり、
        // 角付近のシルエットが四角いまま → ぼかし後も角に影が残る。穴マスクで絞れば
        // 角はシルエットの外になり、ぼかしの裾しか乗らないので中心付近より暗くなる。
        let frame = frame_with_rounded_ring();
        let p = ShadowParams::for_frame(100, 200);
        let mask = screen_mask(&frame, HOLE);
        let layer = shadow_layer(&frame, HOLE, &mask, &p);

        let center = layer.get_pixel(p.padding + HOLE.x + 30, p.padding + p.offset_y + HOLE.y + 70);
        assert!(center[3] > 0, "画面中央のシルエットには影がある (alpha={})", center[3]);

        let corner = layer.get_pixel(p.padding + HOLE.x, p.padding + p.offset_y + HOLE.y);
        let inside_diagonal =
            layer.get_pixel(p.padding + HOLE.x + 12, p.padding + p.offset_y + HOLE.y + 12);
        assert!(
            corner[3] < inside_diagonal[3],
            "外接矩形の角は丸い穴マスクの外（ぼかしの裾のみ）なので、対角線上 12px 内側より暗い \
             (corner alpha={}, inside alpha={})",
            corner[3],
            inside_diagonal[3]
        );
    }

    #[test]
    fn parse_hex_color_accepts_3_and_6_digits() {
        assert_eq!(parse_hex_color("#fff").unwrap(), Rgba([255, 255, 255, 255]));
        assert_eq!(parse_hex_color("#1a2B3c").unwrap(), Rgba([26, 43, 60, 255]));
        assert_eq!(parse_hex_color("  #000 ").unwrap(), Rgba([0, 0, 0, 255]), "前後の空白は無視");
    }

    #[test]
    fn parse_hex_color_rejects_invalid() {
        for s in ["fff", "#ggg", "#12345", "", "#", "#1234567", "white"] {
            let err = parse_hex_color(s).unwrap_err();
            assert!(err.contains("Invalid background color"), "{s}: {err}");
        }
    }

    /// 標準的な角丸矩形判定: まず外接矩形内かを見て、角の正方形領域にいる場合だけ
    /// 角の円弧中心までの距離を半径と比較する。
    fn inside_rounded(x: f32, y: f32, r: &Rect, radius: f32) -> bool {
        let (left, top) = (r.x as f32, r.y as f32);
        let (right, bottom) = (r.right() as f32, r.bottom() as f32);
        if x < left || x > right || y < top || y > bottom {
            return false;
        }
        let corner_x = if x < left + radius {
            left + radius
        } else if x > right - radius {
            right - radius
        } else {
            x
        };
        let corner_y = if y < top + radius {
            top + radius
        } else if y > bottom - radius {
            bottom - radius
        } else {
            y
        };
        if corner_x == x || corner_y == y {
            true
        } else {
            let (dx, dy) = (x - corner_x, y - corner_y);
            dx * dx + dy * dy <= radius * radius
        }
    }

    /// 100x200 のフレーム。透明地に、HOLE を囲む幅 6px の丸角リング（半径 26）だけが不透明。
    /// リングの内側（HOLE 自体、半径 20）と外側は透明のまま。
    fn frame_with_rounded_ring() -> RgbaImage {
        let mut f = solid(100, 200, [0, 0, 0, 0]);
        let ring_outer =
            Rect { x: HOLE.x - 6, y: HOLE.y - 6, width: HOLE.width + 12, height: HOLE.height + 12 };
        for y in 0..200u32 {
            for x in 0..100u32 {
                let (xf, yf) = (x as f32 + 0.5, y as f32 + 0.5);
                let in_ring = inside_rounded(xf, yf, &ring_outer, 26.0)
                    && !inside_rounded(xf, yf, &HOLE, 20.0);
                if in_ring {
                    f.put_pixel(x, y, Rgba(BEZEL));
                }
            }
        }
        f
    }

    #[test]
    fn screen_mask_follows_rounded_hole() {
        let frame = frame_with_rounded_ring();
        let m = screen_mask(&frame, HOLE);
        assert!(m[(70 * 60) + 30], "画面中央は true");
        assert!(!m[0], "外接矩形の左上角（弧の外）は false");
        assert!(m[(70 * 60) + 0], "左辺中央は true");
    }

    #[test]
    fn screen_mask_survives_opaque_pixel_at_center() {
        // 穴の中央に 9x9 の不透明な装飾（ベゼル色）を重ねても、起点探索が中央近傍の
        // 通行可能画素を見つけてフラッドフィルするため、クリップは無効化されない。
        let mut frame = frame_with_rounded_ring();
        let (center_x, center_y) = (HOLE.x + HOLE.width / 2, HOLE.y + HOLE.height / 2);
        for y in (center_y - 4)..=(center_y + 4) {
            for x in (center_x - 4)..=(center_x + 4) {
                frame.put_pixel(x, y, Rgba(BEZEL));
            }
        }
        let m = screen_mask(&frame, HOLE);
        assert!(!m[0], "外接矩形の左上角（弧の外）は false");
        assert!(m[(70 * 60) + 0], "左辺中央は true（クリップは維持される）");
        assert!(!m[(70 * 60) + 30], "中央画素自体は不透明な装飾なので false");

        let shot = solid(60, 140, [200, 0, 0, 255]);
        let out = compose_frame(&shot, &frame, HOLE, false, None);
        assert_eq!(out.get_pixel(HOLE.x, HOLE.y)[3], 0, "穴の外接矩形の角はクリップされ透明");
        assert_eq!(
            out.get_pixel(HOLE.x + 30, HOLE.y + 20).0,
            [200, 0, 0, 255],
            "装飾から離れた穴の画素は赤のまま"
        );
    }

    #[test]
    fn screenshot_corners_are_clipped_to_the_hole() {
        let frame = frame_with_rounded_ring();
        let shot = solid(60, 140, [200, 0, 0, 255]);
        let out = compose_frame(&shot, &frame, HOLE, false, None);
        assert_eq!(out.get_pixel(HOLE.x, HOLE.y)[3], 0, "穴の外接矩形の角はクリップされ透明");
        assert_eq!(out.get_pixel(HOLE.x + 30, HOLE.y + 70).0, [200, 0, 0, 255], "中央は赤のまま");
        assert_eq!(out.get_pixel(HOLE.x - 3, HOLE.y + 70).0, BEZEL, "リング部分はリングの色");
    }

    #[test]
    fn screen_mask_is_full_rect_when_hole_is_square() {
        let m = screen_mask(&frame_with_hole(), HOLE);
        assert!(m.iter().all(|&v| v), "四角い穴なら全画素 true（クリップしない）");
    }

    #[test]
    fn background_fills_transparent_areas_and_shadow_darkens_it() {
        let shot = solid(60, 140, [200, 0, 0, 255]);
        let mut frame = frame_with_hole();
        frame.put_pixel(0, 0, Rgba([0, 0, 0, 0])); // フレーム外周に透明画素を 1 つ

        // 背景なし: 従来どおり透明
        let out = compose_frame(&shot, &frame, HOLE, false, None);
        assert_eq!(out.get_pixel(0, 0)[3], 0);

        // 白背景・影なし: 透明画素が白に、画面とベゼルは不変
        let white = Rgba([255, 255, 255, 255]);
        let out = compose_frame(&shot, &frame, HOLE, false, Some(white));
        assert_eq!(out.get_pixel(0, 0).0, [255, 255, 255, 255]);
        assert_eq!(out.get_pixel(50, 100).0, [200, 0, 0, 255]);
        assert_eq!(out.get_pixel(5, 5).0, BEZEL);

        // 白背景・影あり: 四隅は純白、本体下は白が暗くなる（黒い影が乗る）
        let p = ShadowParams::for_frame(100, 200);
        let out = compose_frame(&shot, &frame, HOLE, true, Some(white));
        assert_eq!(out.get_pixel(0, 0).0, [255, 255, 255, 255]);
        let below = out.get_pixel(p.padding + 50, p.padding + 200 + 1);
        assert_eq!(below[3], 255, "背景ありなら不透明");
        assert!(below[0] < 250 && below[0] == below[1] && below[1] == below[2], "白の上に黒の影 → 灰色: {:?}", below);
    }
}
