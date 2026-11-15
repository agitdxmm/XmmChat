# -*- coding: utf-8 -*-
"""量一下咪的天 窗口与屏幕/工作区的尺寸关系，用于判断窗口是否超出屏幕。"""
import ctypes
from ctypes import wintypes

user32 = ctypes.windll.user32
try:
    ctypes.windll.shcore.SetProcessDpiAwareness(2)
except Exception:
    user32.SetProcessDPIAware()


class RECT(ctypes.Structure):
    _fields_ = [
        ("left", wintypes.LONG),
        ("top", wintypes.LONG),
        ("right", wintypes.LONG),
        ("bottom", wintypes.LONG),
    ]


SPI_GETWORKAREA = 0x0030


def main() -> None:
    hwnd = user32.FindWindowW(None, "咪的天")
    if not hwnd:
        raise SystemExit("没找到窗口")

    wr = RECT()
    user32.GetWindowRect(hwnd, ctypes.byref(wr))
    cr = RECT()
    user32.GetClientRect(hwnd, ctypes.byref(cr))

    work = RECT()
    user32.SystemParametersInfoW(SPI_GETWORKAREA, 0, ctypes.byref(work), 0)
    screen_w = user32.GetSystemMetrics(0)
    screen_h = user32.GetSystemMetrics(1)

    print(f"屏幕(物理)      : {screen_w} x {screen_h}")
    print(f"工作区(物理)    : {work.right - work.left} x {work.bottom - work.top}"
          f"  @({work.left},{work.top})")
    print(f"窗口(物理)      : {wr.right - wr.left} x {wr.bottom - wr.top}"
          f"  @({wr.left},{wr.top})")
    print(f"客户区(物理)    : {cr.right} x {cr.bottom}")
    print(f"窗口底边        : {wr.bottom}  (工作区底边 {work.bottom})")
    if wr.bottom > work.bottom:
        print(">>> 窗口超出工作区底部 %d px" % (wr.bottom - work.bottom))
    else:
        print(">>> 窗口完整可见")


if __name__ == "__main__":
    main()
