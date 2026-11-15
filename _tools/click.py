# -*- coding: utf-8 -*-
"""往屏幕坐标点一下鼠标（纯 ctypes，无第三方依赖）

用法:
    python click.py <x> <y> [次数]
"""
import ctypes
import sys
import time

user32 = ctypes.windll.user32

MOUSEEVENTF_LEFTDOWN = 0x0002
MOUSEEVENTF_LEFTUP = 0x0004


def main() -> None:
    if len(sys.argv) < 3:
        raise SystemExit("用法: python click.py <x> <y> [次数]")
    x, y = int(sys.argv[1]), int(sys.argv[2])
    times = int(sys.argv[3]) if len(sys.argv) > 3 else 1

    try:
        ctypes.windll.shcore.SetProcessDpiAwareness(2)
    except Exception:
        user32.SetProcessDPIAware()

    for _ in range(times):
        user32.SetCursorPos(x, y)
        time.sleep(0.2)
        user32.mouse_event(MOUSEEVENTF_LEFTDOWN, 0, 0, 0, 0)
        time.sleep(0.06)
        user32.mouse_event(MOUSEEVENTF_LEFTUP, 0, 0, 0, 0)
        time.sleep(0.4)
    print(f"已在 ({x}, {y}) 点击 {times} 次")


if __name__ == "__main__":
    main()
