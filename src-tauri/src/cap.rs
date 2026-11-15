//! 屏幕捕获（远程查看用）。
//!
//! Windows 走 GDI：`BitBlt` 抓主屏 → 按块平均缩到目标宽度 → 手写 PNG。
//! 不引第三方图像库，只依赖 `flate2`（纯 Rust 的 zlib）。
//!
//! 输出是 base64 的 PNG，前端直接塞进 `<img src="data:image/png;base64,...">`，
//! 再通过 WebSocket 发给对端 —— 于是「远程看屏幕」不依赖任何 WebRTC 屏幕捕获
//! 权限（WebView2 对 getDisplayMedia 支持并不完整）。

use base64::Engine;

#[tauri::command]
pub fn screen_grab(max_width: Option<u32>) -> Result<String, String> {
    let width = max_width.unwrap_or(960).clamp(320, 2560);
    let png = grab(width)?;
    Ok(base64::engine::general_purpose::STANDARD.encode(png))
}

#[cfg(windows)]
fn grab(max_width: u32) -> Result<Vec<u8>, String> {
    imp::grab(max_width)
}

#[cfg(not(windows))]
fn grab(_max_width: u32) -> Result<Vec<u8>, String> {
    Err("当前平台暂不支持屏幕捕获".into())
}

// ============================================================
// PNG 编码器（RGB8）
// ============================================================

fn crc_table() -> [u32; 256] {
    let mut table = [0u32; 256];
    for n in 0..256u32 {
        let mut c = n;
        for _ in 0..8 {
            c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
        }
        table[n as usize] = c;
    }
    table
}

fn crc32(data: &[u8]) -> u32 {
    let table = crc_table();
    let mut c = 0xFFFF_FFFFu32;
    for b in data {
        c = table[((c ^ *b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

fn push_chunk(out: &mut Vec<u8>, kind: &[u8; 4], payload: &[u8]) {
    out.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    let start = out.len();
    out.extend_from_slice(kind);
    out.extend_from_slice(payload);
    let crc = crc32(&out[start..]);
    out.extend_from_slice(&crc.to_be_bytes());
}

/// rgb: 长度 = w*h*3，行优先
fn encode_png(rgb: &[u8], w: usize, h: usize) -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    // 每行前加一个 filter byte（0 = None）
    let mut raw = Vec::with_capacity(h * (w * 3 + 1));
    for row in 0..h {
        raw.push(0);
        raw.extend_from_slice(&rgb[row * w * 3..(row + 1) * w * 3]);
    }

    let mut enc = ZlibEncoder::new(Vec::new(), Compression::fast());
    enc.write_all(&raw).expect("zlib 写入失败");
    let z = enc.finish().expect("zlib 结束失败");

    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&(w as u32).to_be_bytes());
    ihdr.extend_from_slice(&(h as u32).to_be_bytes());
    ihdr.push(8); // bit depth
    ihdr.push(2); // color type: truecolor
    ihdr.push(0); // compression
    ihdr.push(0); // filter
    ihdr.push(0); // interlace

    let mut out = Vec::with_capacity(z.len() + 64);
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
    push_chunk(&mut out, b"IHDR", &ihdr);
    push_chunk(&mut out, b"IDAT", &z);
    push_chunk(&mut out, b"IEND", &[]);
    out
}

/// 按块平均缩放：BGRA 源 → RGB 目标
fn resize_bgra_to_rgb(
    src: &[u8],
    sw: usize,
    sh: usize,
    dw: usize,
    dh: usize,
) -> Vec<u8> {
    let mut out = vec![0u8; dw * dh * 3];
    for dy in 0..dh {
        let y0 = dy * sh / dh;
        let y1 = ((dy + 1) * sh / dh).max(y0 + 1).min(sh);
        for dx in 0..dw {
            let x0 = dx * sw / dw;
            let x1 = ((dx + 1) * sw / dw).max(x0 + 1).min(sw);
            let mut b: u32 = 0;
            let mut g: u32 = 0;
            let mut r: u32 = 0;
            let mut n: u32 = 0;
            for y in y0..y1 {
                let base = y * sw * 4;
                for x in x0..x1 {
                    let i = base + x * 4;
                    b += src[i] as u32;
                    g += src[i + 1] as u32;
                    r += src[i + 2] as u32;
                    n += 1;
                }
            }
            if n == 0 {
                n = 1;
            }
            let o = (dy * dw + dx) * 3;
            out[o] = (r / n) as u8;
            out[o + 1] = (g / n) as u8;
            out[o + 2] = (b / n) as u8;
        }
    }
    out
}

#[cfg(windows)]
mod imp {
    use super::*;
    use std::ffi::c_void;
    use windows_sys::Win32::Graphics::Gdi::{
        BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC,
        GetDIBits, ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
        HDC, HGDIOBJ, RGBQUAD, SRCCOPY,
    };

    // 取「桌面物理分辨率」，不受 DPI 虚拟化影响
    const DESKTOPHORZRES: i32 = 118;
    const DESKTOPVERTRES: i32 = 117;

    #[link(name = "gdi32")]
    extern "system" {
        fn GetDeviceCaps(hdc: HDC, index: i32) -> i32;
    }

    pub fn grab(max_width: u32) -> Result<Vec<u8>, String> {
        unsafe {
            let null_hwnd = std::ptr::null_mut::<c_void>();
            let screen = GetDC(null_hwnd);
            if screen.is_null() {
                return Err("GetDC 失败".into());
            }
            let sw = GetDeviceCaps(screen, DESKTOPHORZRES).max(1) as usize;
            let sh = GetDeviceCaps(screen, DESKTOPVERTRES).max(1) as usize;

            let mem = CreateCompatibleDC(screen);
            let bmp = CreateCompatibleBitmap(screen, sw as i32, sh as i32);
            if mem.is_null() || bmp.is_null() {
                ReleaseDC(null_hwnd, screen);
                return Err("创建离屏位图失败".into());
            }
            let old: HGDIOBJ = SelectObject(mem, bmp);
            let ok = BitBlt(mem, 0, 0, sw as i32, sh as i32, screen, 0, 0, SRCCOPY);

            let mut result = Err("BitBlt 失败".to_string());
            if ok != 0 {
                let mut buf = vec![0u8; sw * sh * 4];
                let mut bi = BITMAPINFO {
                    bmiHeader: BITMAPINFOHEADER {
                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                        biWidth: sw as i32,
                        biHeight: -(sh as i32), // 负值 = 自上而下
                        biPlanes: 1,
                        biBitCount: 32,
                        biCompression: BI_RGB,
                        biSizeImage: 0,
                        biXPelsPerMeter: 0,
                        biYPelsPerMeter: 0,
                        biClrUsed: 0,
                        biClrImportant: 0,
                    },
                    bmiColors: [RGBQUAD {
                        rgbBlue: 0,
                        rgbGreen: 0,
                        rgbRed: 0,
                        rgbReserved: 0,
                    }],
                };
                let lines = GetDIBits(
                    mem,
                    bmp,
                    0,
                    sh as u32,
                    buf.as_mut_ptr() as *mut c_void,
                    &mut bi,
                    DIB_RGB_COLORS,
                );
                if lines != 0 {
                    let dw = (max_width as usize).min(sw).max(1);
                    let dh = ((sh as f64) * (dw as f64) / (sw as f64)).round().max(1.0) as usize;
                    let rgb = if dw == sw && dh == sh {
                        // 不缩放，直接转
                        let mut v = vec![0u8; sw * sh * 3];
                        for i in 0..sw * sh {
                            v[i * 3] = buf[i * 4 + 2];
                            v[i * 3 + 1] = buf[i * 4 + 1];
                            v[i * 3 + 2] = buf[i * 4];
                        }
                        v
                    } else {
                        resize_bgra_to_rgb(&buf, sw, sh, dw, dh)
                    };
                    result = Ok(encode_png(&rgb, dw, dh));
                }
            }

            SelectObject(mem, old);
            DeleteObject(bmp);
            DeleteDC(mem);
            ReleaseDC(null_hwnd, screen);
            result
        }
    }
}
