# -*- coding: utf-8 -*-
"""截取小喵咪ZIP 窗口（不依赖前台），存成 PNG。

用 PrintWindow + PW_RENDERFULLCONTENT 直接从窗口句柄渲染，
**不受其它窗口遮挡影响** —— 之前用屏幕 BitBlt 时，正在全屏跑的游戏会把画面抢掉。

用法：python focus_shot.py <输出路径> [--client]
      默认截整个窗口（含标题栏）；加 --client 只截客户区
"""
import ctypes
import struct
import sys
import time
import zlib
from ctypes import wintypes

user32 = ctypes.windll.user32
gdi32 = ctypes.windll.gdi32
try:
    ctypes.windll.shcore.SetProcessDpiAwareness(2)
except Exception:
    user32.SetProcessDPIAware()

TITLE = "咪的天"
PW_RENDERFULLCONTENT = 0x00000002


class RECT(ctypes.Structure):
    _fields_ = [
        ("left", wintypes.LONG),
        ("top", wintypes.LONG),
        ("right", wintypes.LONG),
        ("bottom", wintypes.LONG),
    ]


class BITMAPINFOHEADER(ctypes.Structure):
    _fields_ = [
        ("biSize", wintypes.DWORD),
        ("biWidth", wintypes.LONG),
        ("biHeight", wintypes.LONG),
        ("biPlanes", wintypes.WORD),
        ("biBitCount", wintypes.WORD),
        ("biCompression", wintypes.DWORD),
        ("biSizeImage", wintypes.DWORD),
        ("biXPelsPerMeter", wintypes.LONG),
        ("biYPelsPerMeter", wintypes.LONG),
        ("biClrUsed", wintypes.DWORD),
        ("biClrImportant", wintypes.DWORD),
    ]


def write_png(path: str, w: int, h: int, rgba: bytes) -> None:
    raw = bytearray()
    stride = w * 4
    for y in range(h):
        raw.append(0)
        raw += rgba[y * stride : (y + 1) * stride]

    def chunk(tag: bytes, data: bytes) -> bytes:
        return (
            struct.pack(">I", len(data))
            + tag
            + data
            + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
        )

    png = (
        b"\x89PNG\r\n\x1a\n"
        + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
        + chunk(b"IDAT", zlib.compress(bytes(raw), 6))
        + chunk(b"IEND", b"")
    )
    with open(path, "wb") as f:
        f.write(png)


def main() -> None:
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    out = args[0] if args else "shot_app.png"
    client_only = "--client" in sys.argv

    hwnd = user32.FindWindowW(None, TITLE)
    if not hwnd:
        raise SystemExit("没找到窗口：" + TITLE)

    # 先唤醒，避免最小化的窗口抓到空白
    user32.ShowWindow(hwnd, 9 if user32.IsIconic(hwnd) else 5)
    time.sleep(0.8)

    if client_only:
        r = RECT()
        user32.GetClientRect(hwnd, ctypes.byref(r))
        w, h = r.right, r.bottom
    else:
        r = RECT()
        user32.GetWindowRect(hwnd, ctypes.byref(r))
        w, h = r.right - r.left, r.bottom - r.top

    hdc_win = user32.GetDC(0)
    hdc_mem = gdi32.CreateCompatibleDC(hdc_win)
    hbmp = gdi32.CreateCompatibleBitmap(hdc_win, w, h)
    gdi32.SelectObject(hdc_mem, hbmp)

    # 直接从窗口渲染，绕过遮挡
    if user32.PrintWindow(hwnd, hdc_mem, PW_RENDERFULLCONTENT):
        mode = "PrintWindow"
    else:
        pt = wintypes.POINT(0, 0)
        user32.ClientToScreen(hwnd, ctypes.byref(pt))
        gdi32.BitBlt(hdc_mem, 0, 0, w, h, hdc_win, pt.x, pt.y, 0x00CC0020)
        mode = "BitBlt 回退"

    bi = BITMAPINFOHEADER()
    bi.biSize = ctypes.sizeof(BITMAPINFOHEADER)
    bi.biWidth = w
    bi.biHeight = -h  # 自上而下
    bi.biPlanes = 1
    bi.biBitCount = 32
    bi.biCompression = 0

    buf = ctypes.create_string_buffer(w * h * 4)
    gdi32.GetDIBits(hdc_mem, hbmp, 0, h, buf, ctypes.byref(bi), 0)

    bgra = buf.raw
    rgba = bytearray(len(bgra))
    for i in range(0, len(bgra), 4):
        rgba[i] = bgra[i + 2]
        rgba[i + 1] = bgra[i + 1]
        rgba[i + 2] = bgra[i]
        rgba[i + 3] = 255

    write_png(out, w, h, bytes(rgba))

    gdi32.DeleteObject(hbmp)
    gdi32.DeleteDC(hdc_mem)
    user32.ReleaseDC(0, hdc_win)

    print(f"已截图 {mode}  {w}x{h} -> {out}")


if __name__ == "__main__":
    main()
