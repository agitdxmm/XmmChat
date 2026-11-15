# -*- coding: utf-8 -*-
"""纯 ctypes 屏幕截图 -> PNG（不依赖 PIL / 第三方库）

用法:
    python shot.py <输出文件.png>

说明:
    为了让 Read 工具能看图，这里用 zlib 手写了一个最小 PNG 编码器。
"""
import ctypes
import struct
import sys
import zlib
from ctypes import wintypes

user32 = ctypes.windll.user32
gdi32 = ctypes.windll.gdi32

SRCCOPY = 0x00CC0020
SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN = 76, 77
SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN = 78, 79


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


def _chunk(tag: bytes, data: bytes) -> bytes:
    return (
        struct.pack(">I", len(data))
        + tag
        + data
        + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)
    )


def write_png(path: str, w: int, h: int, bgra: bytes) -> None:
    rows = []
    stride = w * 4
    for j in range(h):
        row = bytearray(bgra[j * stride:(j + 1) * stride])
        # BGRA -> RGBA
        row[0::4], row[2::4] = row[2::4], row[0::4]
        rows.append(b"\x00" + bytes(row))
    raw = b"".join(rows)

    out = b"\x89PNG\r\n\x1a\n"
    out += _chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 6, 0, 0, 0))
    out += _chunk(b"IDAT", zlib.compress(raw, 6))
    out += _chunk(b"IEND", b"")
    with open(path, "wb") as f:
        f.write(out)


def capture(path: str) -> None:
    # 高清屏下让坐标与像素一一对应
    try:
        ctypes.windll.shcore.SetProcessDpiAwareness(2)
    except Exception:
        user32.SetProcessDPIAware()

    x = user32.GetSystemMetrics(SM_XVIRTUALSCREEN)
    y = user32.GetSystemMetrics(SM_YVIRTUALSCREEN)
    w = user32.GetSystemMetrics(SM_CXVIRTUALSCREEN)
    h = user32.GetSystemMetrics(SM_CYVIRTUALSCREEN)
    if w <= 0 or h <= 0:
        raise SystemExit("拿不到屏幕尺寸（可能没有桌面会话）")

    hdc = user32.GetDC(0)
    mdc = gdi32.CreateCompatibleDC(hdc)
    bmp = gdi32.CreateCompatibleBitmap(hdc, w, h)
    old = gdi32.SelectObject(mdc, bmp)
    try:
        if not gdi32.BitBlt(mdc, 0, 0, w, h, hdc, x, y, SRCCOPY):
            raise SystemExit("BitBlt 失败")

        bi = BITMAPINFOHEADER()
        bi.biSize = ctypes.sizeof(BITMAPINFOHEADER)
        bi.biWidth = w
        bi.biHeight = -h  # 负数 = 自上而下
        bi.biPlanes = 1
        bi.biBitCount = 32
        bi.biCompression = 0  # BI_RGB

        buf = ctypes.create_string_buffer(w * h * 4)
        if not gdi32.GetDIBits(mdc, bmp, 0, h, buf, ctypes.byref(bi), 0):
            raise SystemExit("GetDIBits 失败")
        write_png(path, w, h, buf.raw)
    finally:
        gdi32.SelectObject(mdc, old)
        gdi32.DeleteObject(bmp)
        gdi32.DeleteDC(mdc)
        user32.ReleaseDC(0, hdc)

    print(f"已截图 {w}x{h} -> {path}")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        raise SystemExit("用法: python shot.py <输出.png>")
    capture(sys.argv[1])
