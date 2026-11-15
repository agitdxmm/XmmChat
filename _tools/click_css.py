# -*- coding: utf-8 -*-
"""按「CSS 坐标」点击咪的天 窗口内的元素。

设计稿宽度 1260px（见 tauri.conf.json 的 window.width），
脚本会读取窗口客户区实际尺寸算出缩放比，再换算成屏幕坐标点击。

用法:
    python click_css.py <css_x> <css_y>
"""
import ctypes
import sys
import time
from ctypes import wintypes

user32 = ctypes.windll.user32

DESIGN_W = 1260.0
TITLE = "咪的天"

MOUSEEVENTF_LEFTDOWN = 0x0002
MOUSEEVENTF_LEFTUP = 0x0004


class RECT(ctypes.Structure):
    _fields_ = [
        ("left", wintypes.LONG),
        ("top", wintypes.LONG),
        ("right", wintypes.LONG),
        ("bottom", wintypes.LONG),
    ]


class POINT(ctypes.Structure):
    _fields_ = [("x", wintypes.LONG), ("y", wintypes.LONG)]


def main() -> None:
    if len(sys.argv) < 3:
        raise SystemExit("用法: python click_css.py <css_x> <css_y>")
    css_x, css_y = float(sys.argv[1]), float(sys.argv[2])

    try:
        ctypes.windll.shcore.SetProcessDpiAwareness(2)
    except Exception:
        user32.SetProcessDPIAware()

    hwnd = user32.FindWindowW(None, TITLE)
    if not hwnd:
        raise SystemExit(f"没找到标题为 {TITLE} 的窗口")

    rc = RECT()
    if not user32.GetClientRect(hwnd, ctypes.byref(rc)):
        raise SystemExit("GetClientRect 失败")
    client_w = rc.right - rc.left

    origin = POINT(0, 0)
    if not user32.ClientToScreen(hwnd, ctypes.byref(origin)):
        raise SystemExit("ClientToScreen 失败")

    scale = client_w / DESIGN_W
    sx = int(origin.x + css_x * scale)
    sy = int(origin.y + css_y * scale)

    print(f"客户区宽 {client_w}px, 缩放 {scale:.3f} -> 屏幕 ({sx}, {sy})")

    user32.SetForegroundWindow(hwnd)
    time.sleep(0.3)
    user32.SetCursorPos(sx, sy)
    time.sleep(0.2)
    user32.mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0)
    time.sleep(0.06)
    user32.mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0)
    time.sleep(0.4)


if __name__ == "__main__":
    main()
