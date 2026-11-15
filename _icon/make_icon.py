"""生成「咪的天」应用图标：渐变圆角底 + 液态玻璃感气泡 + 猫脸。

纯 PIL，无外部素材。输出 1024x1024 PNG，交给 `tauri icon` 生成全套尺寸。
"""

from PIL import Image, ImageDraw, ImageFilter, ImageOps
import os

S = 1024
OUT = os.path.join(os.path.dirname(os.path.abspath(__file__)), "icon.png")

# ---------------- 背景渐变 ----------------

grad = Image.radial_gradient("L").resize((S, S), Image.BICUBIC)
# 中心亮（淡紫）→ 边缘深（靛紫）
bg = ImageOps.colorize(grad, black="#c7b9ff", white="#5b21b6")
bg = bg.convert("RGBA")

# 叠一层斜向的粉/薄荷光晕，让颜色不那么单调
glow = Image.new("RGBA", (S, S), (0, 0, 0, 0))
gd = ImageDraw.Draw(glow)
gd.ellipse([S * 0.45, -S * 0.25, S * 1.35, S * 0.6], fill=(240, 171, 252, 120))
gd.ellipse([-S * 0.35, S * 0.45, S * 0.55, S * 1.35], fill=(94, 234, 212, 105))
glow = glow.filter(ImageFilter.GaussianBlur(S * 0.11))
bg = Image.alpha_composite(bg, glow)

# ---------------- 圆角遮罩 ----------------

RADIUS = int(S * 0.235)
mask = Image.new("L", (S, S), 0)
ImageDraw.Draw(mask).rounded_rectangle([0, 0, S - 1, S - 1], radius=RADIUS, fill=255)

canvas = Image.new("RGBA", (S, S), (0, 0, 0, 0))
canvas.paste(bg, (0, 0), mask)

# ---------------- 气泡 ----------------

bw, bh = int(S * 0.62), int(S * 0.50)
bx, by = (S - bw) // 2, int(S * 0.22)
br = int(bh * 0.34)

bubble = Image.new("RGBA", (S, S), (0, 0, 0, 0))
bd = ImageDraw.Draw(bubble)

# 气泡主体：白色，略带透明度，做出玻璃感
bd.rounded_rectangle([bx, by, bx + bw, by + bh], radius=br, fill=(255, 255, 255, 242))

# 左下角的小尾巴
tail = [
    (bx + int(bw * 0.24), by + bh - int(bh * 0.06)),
    (bx + int(bw * 0.10), by + bh + int(bh * 0.20)),
    (bx + int(bw * 0.42), by + bh - int(bh * 0.02)),
]
bd.polygon(tail, fill=(255, 255, 255, 242))

# 内部一层很淡的渐变，避免大块死白
inner = Image.new("RGBA", (S, S), (0, 0, 0, 0))
idr = ImageDraw.Draw(inner)
idr.rounded_rectangle(
    [bx + 10, by + 10, bx + bw - 10, by + bh - 10],
    radius=br - 10,
    fill=(167, 139, 250, 26),
)
bubble = Image.alpha_composite(bubble, inner)

canvas = Image.alpha_composite(canvas, bubble)

# ---------------- 脸 ----------------

face = Image.new("RGBA", (S, S), (0, 0, 0, 0))
fd = ImageDraw.Draw(face)

eye_r = int(S * 0.038)
eye_y = by + int(bh * 0.44)
l_eye_x = bx + int(bw * 0.34)
r_eye_x = bx + int(bw * 0.66)

for ex in (l_eye_x, r_eye_x):
    fd.ellipse([ex - eye_r, eye_y - eye_r, ex + eye_r, eye_y + eye_r], fill=(28, 22, 48, 255))

# 眼睛里点一点高光
for ex in (l_eye_x, r_eye_x):
    hr = int(eye_r * 0.34)
    fd.ellipse(
        [ex - hr + int(eye_r * 0.2), eye_y - hr - int(eye_r * 0.2),
         ex + hr + int(eye_r * 0.2), eye_y + hr - int(eye_r * 0.2)],
        fill=(255, 255, 255, 235),
    )

# 微笑
smile_box = [
    bx + int(bw * 0.34),
    by + int(bh * 0.46),
    bx + int(bw * 0.66),
    by + int(bh * 0.72),
]
fd.arc(smile_box, start=18, end=162, fill=(28, 22, 48, 255), width=int(S * 0.017))

canvas = Image.alpha_composite(canvas, face)

# ---------------- 顶部高光 ----------------

spec = Image.new("RGBA", (S, S), (0, 0, 0, 0))
sd = ImageDraw.Draw(spec)
sd.rounded_rectangle(
    [bx + int(bw * 0.08), by + int(bh * 0.05), bx + int(bw * 0.92), by + int(bh * 0.26)],
    radius=int(bh * 0.14),
    fill=(255, 255, 255, 92),
)
spec = spec.filter(ImageFilter.GaussianBlur(S * 0.018))
canvas = Image.alpha_composite(canvas, spec)

# ---------------- 外发光边 ----------------

rim = Image.new("RGBA", (S, S), (0, 0, 0, 0))
rd = ImageDraw.Draw(rim)
rd.rounded_rectangle(
    [1, 1, S - 2, S - 2],
    radius=RADIUS,
    outline=(255, 255, 255, 60),
    width=max(2, int(S * 0.004)),
)
canvas = Image.alpha_composite(canvas, rim)

canvas.save(OUT)
print("written:", OUT, canvas.size)
