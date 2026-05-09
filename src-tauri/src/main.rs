#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use base64::Engine;
use headless_chrome::protocol::cdp::Page::Viewport;
use headless_chrome::protocol::cdp::{Emulation, Page};
use headless_chrome::types::Bounds;
use headless_chrome::{Browser, LaunchOptionsBuilder};
use image::{imageops, ImageOutputFormat, RgbaImage};
use std::io::Cursor;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::command;

static ABORT_FLAG: AtomicBool = AtomicBool::new(false);
const VIEWPORT_HEIGHT: u32 = 1080;

fn set_viewport_metrics(
    tab: &headless_chrome::browser::tab::Tab,
    width: u32,
    height: u32,
) -> Result<(), String> {
    tab.call_method(Emulation::SetDeviceMetricsOverride {
        width,
        height,
        device_scale_factor: 1.0,
        mobile: false,
        scale: None,
        screen_width: Some(width),
        screen_height: Some(height),
        position_x: None,
        position_y: None,
        dont_set_visible_size: None,
        screen_orientation: None,
        viewport: None,
        display_feature: None,
        device_posture: None,
    })
    .map(|_| ())
    .map_err(|e| e.to_string())
}

fn page_metric(
    tab: &headless_chrome::browser::tab::Tab,
    key: &str,
    fallback: f64,
) -> Result<f64, String> {
    let script = format!(
        r#"
        (() => {{
            const doc = document.documentElement;
            const body = document.body;
            const viewportWidth = Math.ceil(window.innerWidth || doc.clientWidth || 0);
            const viewportHeight = Math.ceil(window.innerHeight || doc.clientHeight || 0);
            const scrollWidth = Math.ceil(Math.max(
                viewportWidth,
                doc ? doc.scrollWidth : 0,
                doc ? doc.offsetWidth : 0,
                body ? body.scrollWidth : 0,
                body ? body.offsetWidth : 0
            ));
            const scrollHeight = Math.ceil(Math.max(
                viewportHeight,
                doc ? doc.scrollHeight : 0,
                doc ? doc.offsetHeight : 0,
                body ? body.scrollHeight : 0,
                body ? body.offsetHeight : 0
            ));
            return {{ viewportWidth, viewportHeight, scrollWidth, scrollHeight }}.{};
        }})()
        "#,
        key
    );

    let value = tab
        .evaluate(&script, false)
        .map_err(|e| e.to_string())?
        .value;
    Ok(value.and_then(|v| v.as_f64()).unwrap_or(fallback).max(1.0))
}

fn capture_screenshot(
    tab: &headless_chrome::browser::tab::Tab,
    clip: Option<Viewport>,
    capture_beyond_viewport: bool,
) -> Result<Vec<u8>, String> {
    let data = tab
        .call_method(Page::CaptureScreenshot {
            format: Some(Page::CaptureScreenshotFormatOption::Png),
            quality: None,
            clip,
            from_surface: Some(true),
            capture_beyond_viewport: Some(capture_beyond_viewport),
            optimize_for_speed: None,
        })
        .map_err(|e| e.to_string())?
        .data;

    base64::prelude::BASE64_STANDARD
        .decode(data)
        .map_err(|e| e.to_string())
}

fn wait_for_paint(tab: &headless_chrome::browser::tab::Tab) -> Result<(), String> {
    let _ = tab.evaluate(
        r#"
        new Promise((resolve) => {
            const finish = () => requestAnimationFrame(() => {
                requestAnimationFrame(() => resolve(true));
            });
            const visibleImages = Array.from(document.images || []).filter((img) => {
                const rect = img.getBoundingClientRect();
                return rect.bottom >= 0 && rect.top <= window.innerHeight && rect.width > 0 && rect.height > 0;
            });
            const pending = visibleImages.filter((img) => !img.complete);
            if (pending.length === 0) {
                finish();
                return;
            }
            let remaining = pending.length;
            const done = () => {
                remaining -= 1;
                if (remaining <= 0) finish();
            };
            const timeout = setTimeout(finish, 1200);
            pending.forEach((img) => {
                img.addEventListener('load', done, { once: true });
                img.addEventListener('error', done, { once: true });
            });
            Promise.allSettled(
                visibleImages.map((img) => img.decode ? img.decode() : Promise.resolve())
            ).then(() => {
                clearTimeout(timeout);
                finish();
            }).catch(() => {
                clearTimeout(timeout);
                finish();
            });
        })
        "#,
        true,
    );
    std::thread::sleep(Duration::from_millis(300));
    Ok(())
}

fn capture_fullpage_stitched(
    tab: &headless_chrome::browser::tab::Tab,
    width: f64,
    viewport_height: f64,
    initial_total_height: f64,
) -> Result<Vec<u8>, String> {
    let width_px = width.ceil().max(1.0) as u32;
    let viewport_height_px = viewport_height.ceil().max(1.0) as u32;
    let mut total_height_px = initial_total_height.ceil().max(viewport_height).max(1.0) as u32;
    let mut canvas = RgbaImage::new(width_px, total_height_px);
    let mut next_y = 0_u32;

    loop {
        if ABORT_FLAG.load(Ordering::Relaxed) {
            return Err("ユーザーによってキャプチャが中止されました".to_string());
        }

        let measured_total_height =
            page_metric(tab, "scrollHeight", total_height_px as f64)?.ceil() as u32;
        if measured_total_height > total_height_px {
            let mut expanded = RgbaImage::new(width_px, measured_total_height);
            imageops::overlay(&mut expanded, &canvas, 0, 0);
            canvas = expanded;
            total_height_px = measured_total_height;
        }

        let max_scroll_y = total_height_px.saturating_sub(viewport_height_px);
        let scroll_y = next_y.min(max_scroll_y);
        let scroll_script = format!("window.scrollTo(0, {}); window.scrollY", scroll_y);
        let actual_y = tab
            .evaluate(&scroll_script, false)
            .map_err(|e| e.to_string())?
            .value
            .and_then(|v| v.as_f64())
            .unwrap_or(scroll_y as f64)
            .round()
            .max(0.0) as u32;

        wait_for_paint(tab)?;

        let png_data = capture_screenshot(tab, None, false)?;
        let frame = image::load_from_memory(&png_data)
            .map_err(|e| format!("Failed to decode fullpage slice: {}", e))?
            .into_rgba8();
        let visible_height = total_height_px.saturating_sub(actual_y).min(frame.height());
        let cropped = imageops::crop_imm(&frame, 0, 0, frame.width().min(width_px), visible_height)
            .to_image();
        imageops::overlay(&mut canvas, &cropped, 0, actual_y as i64);

        if actual_y >= max_scroll_y {
            break;
        }
        next_y = actual_y.saturating_add(viewport_height_px);
    }

    let _ = tab.evaluate("window.scrollTo(0, 0)", false);

    let mut out = Cursor::new(Vec::new());
    image::DynamicImage::ImageRgba8(canvas)
        .write_to(&mut out, ImageOutputFormat::Png)
        .map_err(|e| format!("Failed to encode stitched fullpage PNG: {}", e))?;
    Ok(out.into_inner())
}

fn capture_fullpage_expanded_viewport(
    tab: &headless_chrome::browser::tab::Tab,
    width: f64,
    total_height: f64,
) -> Result<Vec<u8>, String> {
    let width_px = width.ceil().max(1.0) as u32;
    let height_px = total_height.ceil().max(1.0) as u32;

    let _ = tab.set_bounds(Bounds::Normal {
        left: None,
        top: None,
        width: Some(width_px as f64),
        height: Some(height_px as f64),
    });
    set_viewport_metrics(tab, width_px, height_px)?;
    let _ = tab.evaluate("window.scrollTo(0, 0)", false);
    wait_for_paint(tab)?;

    capture_screenshot(tab, None, false)
}

#[command]
fn get_default_save_dir() -> String {
    if let Ok(home) = std::env::var("HOME") {
        return format!("{}/Downloads", home);
    }
    "".to_string()
}

#[command]
async fn select_element(url: String) -> Result<String, String> {
    // Launch visible browser
    let browser_result = LaunchOptionsBuilder::default()
        .headless(false)
        .build()
        .map_err(|e| e.to_string());

    let launch_opts = browser_result?;
    let browser = Browser::new(launch_opts).map_err(|e| e.to_string())?;

    let tab = browser.new_tab().map_err(|e| e.to_string())?;

    // Navigate and wait
    tab.navigate_to(&url).map_err(|e| e.to_string())?;
    tab.wait_until_navigated().map_err(|e| e.to_string())?;

    // Inject JS to highlight and select an element
    let injection_script = r#"
        (function() {
            return new Promise((resolve) => {
                const overlay = document.createElement('div');
                overlay.style.position = 'fixed';
                overlay.style.pointerEvents = 'none';
                overlay.style.zIndex = '999999';
                overlay.style.border = '2px solid red';
                overlay.style.backgroundColor = 'rgba(255, 0, 0, 0.2)';
                overlay.style.transition = 'all 0.1s ease';
                document.body.appendChild(overlay);

                let currentTarget = null;

                const mouseMoveHandler = (e) => {
                    const target = e.target;
                    if (target === document.body || target === document.documentElement) return;
                    currentTarget = target;
                    const rect = target.getBoundingClientRect();
                    overlay.style.top = rect.top + 'px';
                    overlay.style.left = rect.left + 'px';
                    overlay.style.width = rect.width + 'px';
                    overlay.style.height = rect.height + 'px';
                };

                const getCssSelector = (el) => {
                    if (!(el instanceof Element)) return;
                    const path = [];
                    while (el.nodeType === Node.ELEMENT_NODE) {
                        let selector = el.nodeName.toLowerCase();
                        if (el.id) {
                            selector += '#' + el.id;
                            path.unshift(selector);
                            break;
                        } else {
                            let sib = el, nth = 1;
                            while ((sib = sib.previousElementSibling)) {
                                if (sib.nodeName.toLowerCase() == selector)
                                   nth++;
                            }
                            if (nth != 1) selector += ":nth-of-type("+nth+")";
                        }
                        path.unshift(selector);
                        el = el.parentNode;
                    }
                    return path.join(" > ");
                };

                const clickHandler = (e) => {
                    if (!currentTarget) return;
                    e.preventDefault();
                    e.stopPropagation();
                    const selector = getCssSelector(currentTarget);
                    
                    document.removeEventListener('mousemove', mouseMoveHandler);
                    document.removeEventListener('click', clickHandler, true);
                    overlay.remove();
                    
                    resolve(selector);
                };

                document.addEventListener('mousemove', mouseMoveHandler);
                document.addEventListener('click', clickHandler, true);
            });
        })();
    "#;

    let result = tab
        .evaluate(injection_script, true)
        .map_err(|e| e.to_string())?;

    // Extract string value from returned Promise evaluation
    if let Some(val) = result.value {
        if val.is_string() {
            if let Some(s) = val.as_str() {
                return Ok(s.to_string());
            }
        }
    }

    Err("Failed to get selector from injected script".to_string())
}

#[command]
fn abort_capture() {
    ABORT_FLAG.store(true, Ordering::Relaxed);
}

#[command]
fn capture_screenshots(
    url: String,
    widths: Vec<u32>,
    mode: String,
    _format: String,
    selector: String,
    save_dir: String,
    duration: u32,
    delay: u32,
    manual_interaction: bool,
    viewport_height: Option<u32>,
) -> Result<(), String> {
    let save_path = PathBuf::from(save_dir);
    ABORT_FLAG.store(false, Ordering::Relaxed);
    let capture_height = viewport_height.unwrap_or(VIEWPORT_HEIGHT).max(1);

    for w in widths {
        if ABORT_FLAG.load(Ordering::Relaxed) {
            return Err("ユーザーによってキャプチャが中止されました".to_string());
        }

        // Launch a new context or resize for each to be safe and avoid stale states
        let launch_opts = LaunchOptionsBuilder::default()
            .headless(false)
            .window_size(Some((w, capture_height)))
            .args(vec![
                std::ffi::OsStr::new("--disable-web-security"),
                std::ffi::OsStr::new("--disable-site-isolation-trials"),
            ])
            .build()
            .map_err(|e| e.to_string())?;

        let browser = Browser::new(launch_opts).map_err(|e| e.to_string())?;
        let tab = browser.new_tab().map_err(|e| e.to_string())?;

        set_viewport_metrics(&tab, w, capture_height)?;
        tab.navigate_to(&url).map_err(|e| e.to_string())?;
        tab.wait_until_navigated().map_err(|e| e.to_string())?;

        if manual_interaction {
            let btn_text = if duration > 0 {
                "録画開始"
            } else {
                "撮影開始"
            };
            let inject_ui = format!(
                r#"
                (function() {{
                    if (window.__rs_overlay_running) return;
                    window.__rs_overlay_running = true;
                    window.__RS_REC_STATUS = 'waiting';

                    let container = document.createElement('div');
                    container.id = 'rs-recorder-ui';
                    container.style.cssText = 'position:fixed;bottom:20px;right:20px;z-index:2147483647;background:#fff;padding:20px;border-radius:12px;box-shadow:0 8px 30px rgba(0,0,0,0.2);font-family:sans-serif;text-align:center;border:1px solid #e5e7eb;display:flex;flex-direction:column;gap:12px;';
                    
                    let msg = document.createElement('div');
                    msg.style.cssText = 'font-size:13px;color:#333;line-height:1.5;font-weight:500;';
                    msg.innerHTML = '画面を操作して非表示の要素を表示したり、<br/>文字を入力してから下のボタンを押してください。';

                    let btnArray = document.createElement('div');
                    btnArray.style.cssText = 'display:flex;gap:10px;justify-content:center;';

                    let btn = document.createElement('button');
                    btn.innerText = '🔴 {} ({}秒後に開始)';
                    btn.style.cssText = 'background:#ef4444;color:white;border:none;padding:12px 20px;border-radius:8px;font-weight:bold;cursor:pointer;font-size:14px;flex:1;';
                    
                    let cancelBtn = document.createElement('button');
                    cancelBtn.innerText = '中止';
                    cancelBtn.style.cssText = 'background:#f3f4f6;color:#4b5563;border:1px solid #d1d5db;padding:12px 20px;border-radius:8px;font-weight:bold;cursor:pointer;font-size:14px;';

                    btn.onclick = () => {{
                        let countdown = {};
                        btn.blur();
                        btn.style.background = '#f59e0b';
                        btn.style.pointerEvents = 'none';
                        container.style.pointerEvents = 'none';
                        cancelBtn.style.display = 'none';
                        msg.innerText = '操作可能になりました。画面を操作してください...';
                        let intv = setInterval(() => {{
                            if (countdown <= 0) {{
                                clearInterval(intv);
                                window.__RS_REC_STATUS = 'recording';

                                let total_duration = {};
                                if (total_duration > 0) {{
                                    container.style.background = 'rgba(0,0,0,0.8)';
                                    btnArray.style.display = 'none';
                                    msg.style.color = '#fff';
                                    msg.style.fontSize = '15px';
                                    msg.style.fontWeight = 'bold';
                                    msg.innerHTML = `<span style="color:#ef4444">●</span> 録画中... 残り ${{total_duration}}秒`;
                                    
                                    let recIntv = setInterval(() => {{
                                        total_duration--;
                                        if (total_duration <= 0) {{
                                            clearInterval(recIntv);
                                            msg.innerHTML = '保存処理中...';
                                        }} else {{
                                            msg.innerHTML = `<span style="color:#ef4444">●</span> 録画中... 残り ${{total_duration}}秒`;
                                        }}
                                    }}, 1000);
                                }} else {{
                                    container.style.display = 'none';
                                }}
                            }} else {{
                                btn.innerText = `⏳ 開始まで ${{countdown}}秒`;
                            }}
                            countdown--;
                        }}, 1000);
                        btn.onclick = null;
                        btn.innerText = `⏳ 開始まで ${{countdown}}秒`;
                        countdown--;
                    }};

                    cancelBtn.onclick = () => {{
                        container.style.display = 'none';
                        window.__RS_REC_STATUS = 'cancel';
                    }};

                    btnArray.appendChild(cancelBtn);
                    btnArray.appendChild(btn);
                    container.appendChild(msg);
                    container.appendChild(btnArray);
                    
                    let target = document.body || document.documentElement;
                    if (target) {{
                        target.appendChild(container);
                    }}
                    
                    // Keep appending just in case a Vue/React app remounts and clears the DOM
                    setInterval(() => {{
                        if (window.__RS_REC_STATUS === 'waiting' && !document.getElementById('rs-recorder-ui')) {{
                            (document.body || document.documentElement).appendChild(container);
                        }}
                    }}, 800);
                }})();
                "#,
                btn_text, delay, delay, duration
            );
            let _ = tab.evaluate(&inject_ui, false);

            loop {
                if ABORT_FLAG.load(Ordering::Relaxed) {
                    return Err("ユーザーによってキャプチャが中止されました".to_string());
                }

                let status = tab
                    .evaluate("window.__RS_REC_STATUS", false)
                    .map_err(|e| e.to_string())?
                    .value
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| "waiting".to_string());

                if status == "recording" {
                    break;
                }
                if status == "cancel" {
                    return Err("ユーザーによってキャプチャが中止されました".to_string());
                }
                std::thread::sleep(Duration::from_millis(200));
            }
        } else {
            // Use user-defined delay to allow page rendering or lazy-load processing silently
            for _ in 0..(delay * 5) {
                if ABORT_FLAG.load(Ordering::Relaxed) {
                    return Err("ユーザーによってキャプチャが中止されました".to_string());
                }
                std::thread::sleep(Duration::from_millis(200));
            }
        }

        if mode == "fullpage" {
            // Scroll down slowly to trigger intersection observers and lazy loading
            let scroll_script = r#"
                new Promise((resolve) => {
                    const distance = Math.max(300, Math.floor((window.innerHeight || 1080) * 0.8));
                    let lastHeight = 0;
                    let stableTicks = 0;
                    let steps = 0;
                    let timer = setInterval(() => {
                        const doc = document.documentElement;
                        const body = document.body;
                        const scrollHeight = Math.max(
                            window.innerHeight || 0,
                            doc ? doc.scrollHeight : 0,
                            doc ? doc.offsetHeight : 0,
                            body ? body.scrollHeight : 0,
                            body ? body.offsetHeight : 0
                        );

                        window.scrollBy(0, distance);
                        steps++;

                        if (scrollHeight === lastHeight) {
                            stableTicks++;
                        } else {
                            stableTicks = 0;
                            lastHeight = scrollHeight;
                        }

                        const reachedBottom = window.scrollY + window.innerHeight >= scrollHeight - 2;

                        if ((reachedBottom && stableTicks >= 2) || steps >= 300) {
                            clearInterval(timer);
                            resolve('done');
                        }
                    }, 100);
                })
            "#;
            let _ = tab.evaluate(scroll_script, true); // true to await the Promise

            // Wait for images at the bottom to finish loading
            std::thread::sleep(Duration::from_secs(1));

            // Scroll back up
            let _ = tab.evaluate("window.scrollTo(0, 0)", false);

            // Wait for fixed headers to reposition
            std::thread::sleep(Duration::from_secs(1));
        }

        let viewport_w = page_metric(&tab, "viewportWidth", w as f64)?;
        let viewport_h = page_metric(&tab, "viewportHeight", capture_height as f64)?;
        let scroll_h = page_metric(&tab, "scrollHeight", viewport_h)?;

        // Keep the visible viewport stable. Full page capture uses captureBeyondViewport instead
        // of resizing the browser to the whole document height.
        let _ = tab.set_bounds(Bounds::Normal {
            left: None,
            top: None,
            width: Some(w as f64),
            height: Some(capture_height as f64),
        });
        set_viewport_metrics(&tab, w, capture_height)?;

        std::thread::sleep(Duration::from_millis(500)); // wait for final layout snap

        let size_label = if viewport_height.is_some() {
            format!("{}x{}", w, capture_height)
        } else {
            format!("{}px", w)
        };
        let file_name = if duration > 0 {
            format!("capture_{}_{}.gif", size_label, mode)
        } else {
            format!("capture_{}_{}.png", size_label, mode)
        };
        let dst = save_path.join(file_name);

        if ABORT_FLAG.load(Ordering::Relaxed) {
            return Err("ユーザーによってキャプチャが中止されました".to_string());
        }

        if duration > 0 {
            // Pre-calculate exact bounds so GIF frames remain identical dimensions.
            // Using headless_chrome get_box_model to get absolute viewport frame
            let expected_clip = if mode == "element" {
                let rect_json = tab.evaluate(
                    &format!(
                        "(function() {{ let el = document.querySelector('{}'); if (!el) return null; let rect = el.getBoundingClientRect(); return {{ x: rect.left + window.scrollX, y: rect.top + window.scrollY, width: rect.width, height: rect.height }}; }})()",
                        selector.replace("'", "\\'")
                    ),
                    false,
                )
                .map_err(|e| e.to_string())?.value;

                if let Some(val) = rect_json {
                    if let Some(rect) = val.as_object() {
                        Some(Viewport {
                            x: rect["x"].as_f64().unwrap_or(0.0),
                            y: rect["y"].as_f64().unwrap_or(0.0),
                            width: rect["width"].as_f64().unwrap_or(100.0),
                            height: rect["height"].as_f64().unwrap_or(100.0),
                            scale: 1.0,
                        })
                    } else {
                        return Err("Element rect evaluation did not return an object".to_string());
                    }
                } else {
                    return Err("Element not found for GIF capture".to_string());
                }
            } else {
                // For both viewport and fullpage in GIF mode, passing `None` allows Chromium to automatically capture exactly the inner rendering surface without out-of-bounds exceptions.
                // It safely avoids Mac OS titlebar height differences.
                None
            };

            let mut file_opt = Some(
                std::fs::File::create(&dst)
                    .map_err(|e| format!("Line {}: Failed to create file: {}", line!(), e))?,
            );
            let mut encoder: Option<gif::Encoder<std::fs::File>> = None;
            let mut first_width = 0;
            let mut first_height = 0;
            let mut frame_count = 0;

            let start_time_total = std::time::Instant::now();

            loop {
                if ABORT_FLAG.load(Ordering::Relaxed) {
                    return Err(format!("Line {}: ユーザーによって中止されました", line!()));
                }

                let elapsed_total = start_time_total.elapsed().as_secs();
                if elapsed_total >= duration as u64 {
                    break;
                }

                let remaining = duration as u64 - elapsed_total;
                let progress_script = format!(
                    "let ui = document.getElementById('rs-recorder-ui'); if (ui) {{ let msg = ui.querySelector('div'); if (msg) msg.textContent = '● 録画・保存中... 残り{}秒 ({}コマ完了)'; }}",
                    remaining, frame_count
                );
                let _ = tab.evaluate(&progress_script, false);

                let frame_start = std::time::Instant::now();

                let png_data = tab
                    .capture_screenshot(
                        headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
                        None,
                        expected_clip.clone(),
                        false,
                    )
                    .map_err(|e| format!("Line {}: Screenshot Error - {}", line!(), e))?;

                let loaded = image::load_from_memory(&png_data)
                    .map_err(|e| format!("Line {}: Image load Error - {}", line!(), e))?;

                // Cap GIF width at 800px to keep encoding fast across all viewport sizes
                const GIF_MAX_WIDTH: u32 = 800;
                let mut img = if loaded.width() > GIF_MAX_WIDTH {
                    let ratio = GIF_MAX_WIDTH as f64 / loaded.width() as f64;
                    let new_height = (loaded.height() as f64 * ratio) as u32;
                    loaded
                        .resize_exact(
                            GIF_MAX_WIDTH,
                            new_height,
                            image::imageops::FilterType::Triangle,
                        )
                        .into_rgba8()
                } else {
                    loaded.into_rgba8()
                };

                if frame_count == 0 {
                    first_width = img.width();
                    first_height = img.height();
                    let file = file_opt.take().unwrap();
                    let mut enc =
                        gif::Encoder::new(file, first_width as u16, first_height as u16, &[])
                            .map_err(|e| format!("Line {}: Encoder create - {}", line!(), e))?;
                    enc.set_repeat(gif::Repeat::Infinite)
                        .map_err(|e| format!("Line {}: Encoder repeat - {}", line!(), e))?;
                    encoder = Some(enc);
                } else if img.width() != first_width || img.height() != first_height {
                    let dynamic_img = image::DynamicImage::ImageRgba8(img);
                    img = dynamic_img
                        .resize_exact(
                            first_width,
                            first_height,
                            image::imageops::FilterType::Nearest,
                        )
                        .into_rgba8();
                }

                let mut frame = gif::Frame::from_rgba_speed(
                    first_width as u16,
                    first_height as u16,
                    &mut img.into_raw(),
                    30,
                );

                // Fixed 3 FPS frame delay (33 units = 330ms in GIF spec)
                frame.delay = 33;

                // Sleep to maintain consistent frame interval
                let elapsed_ms = frame_start.elapsed().as_millis() as u64;
                if elapsed_ms < 330 {
                    std::thread::sleep(Duration::from_millis(330 - elapsed_ms));
                }

                if let Some(ref mut enc) = encoder {
                    enc.write_frame(&frame)
                        .map_err(|e| format!("Line {}: Frame write - {}", line!(), e))?;
                }
                frame_count += 1;
            }
        } else {
            let png_data = match mode.as_str() {
                "element" => {
                    let el = tab.wait_for_element(&selector).map_err(|e| e.to_string())?;
                    el.capture_screenshot(
                        headless_chrome::protocol::cdp::Page::CaptureScreenshotFormatOption::Png,
                    )
                    .map_err(|e| e.to_string())?
                }
                "viewport" => capture_screenshot(
                    &tab,
                    Some(Viewport {
                        x: 0.0,
                        y: 0.0,
                        width: viewport_w,
                        height: viewport_h,
                        scale: 1.0,
                    }),
                    false,
                )?,
                "fullpage" | _ => {
                    if scroll_h <= 16_000.0 {
                        capture_fullpage_expanded_viewport(&tab, viewport_w, scroll_h)?
                    } else {
                        capture_fullpage_stitched(&tab, viewport_w, viewport_h, scroll_h)?
                    }
                }
            };

            std::fs::write(&dst, png_data)
                .map_err(|e| format!("Failed to save image at {:?}: {}", dst, e))?;
        }

        // Drop the browser in a separate thread to avoid blocking the main thread
        std::thread::spawn(move || {
            drop(browser);
        });
    }

    Ok(())
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            get_default_save_dir,
            select_element,
            capture_screenshots,
            abort_capture
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
