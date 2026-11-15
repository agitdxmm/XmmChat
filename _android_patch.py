"""把安卓原生能力补丁打进 Tauri 生成的 Android 工程。

做的事：
  1. 复制 Kotlin 源码（无障碍服务 + Shizuku 插件）
  2. 复制无障碍配置 xml
  3. 往 AndroidManifest.xml 注入 <service> 与 Shizuku <provider>
  4. 往 app/build.gradle.kts 注入 Shizuku 依赖
  5. 往 strings.xml 注入文案

幂等：重复运行不会重复插入（靠标记串判断）。
用法：python _android_patch.py
"""

import os
import re
import shutil
import sys

ROOT = os.path.dirname(os.path.abspath(__file__))
SRC = os.path.join(ROOT, "src-tauri", "android-src")
GEN = os.path.join(ROOT, "src-tauri", "gen", "android")
APP = os.path.join(GEN, "app")
MAIN = os.path.join(APP, "src", "main")

PKG = "com.xmm.chat"
PKG_PATH = PKG.replace(".", os.sep)


def die(msg):
    print("!! " + msg)
    sys.exit(1)


if not os.path.isdir(GEN):
    die("找不到 src-tauri/gen/android —— 先跑 `npx tauri android init`")

manifest = os.path.join(MAIN, "AndroidManifest.xml")
gradle = os.path.join(APP, "build.gradle.kts")
strings = os.path.join(MAIN, "res", "values", "strings.xml")
xml_dir = os.path.join(MAIN, "res", "xml")
java_dir = os.path.join(MAIN, "java", PKG_PATH)

if not os.path.isfile(manifest):
    die("AndroidManifest.xml 不存在，工程结构可能不对")

os.makedirs(java_dir, exist_ok=True)
os.makedirs(xml_dir, exist_ok=True)

# ---------------------------------------------------------------- 1. 源码
for name in ("XmmShellPlugin.kt", "XmmAccessibilityService.kt"):
    shutil.copy2(os.path.join(SRC, name), os.path.join(java_dir, name))
    print("copy  ->", os.path.join(java_dir, name))

shutil.copy2(
    os.path.join(SRC, "xmm_accessibility.xml"),
    os.path.join(xml_dir, "xmm_accessibility.xml"),
)
print("copy  ->", os.path.join(xml_dir, "xmm_accessibility.xml"))

# ---------------------------------------------------------------- 2. Manifest
with open(manifest, "r", encoding="utf-8") as f:
    man = f.read()

if "XmmAccessibilityService" not in man:
    service_xml = """
        <!-- 咪的天：无障碍服务（远程协助用，不提供 shell 能力） -->
        <service
            android:name=".XmmAccessibilityService"
            android:exported="true"
            android:label="@string/xmm_a11y_label"
            android:permission="android.permission.BIND_ACCESSIBILITY_SERVICE">
            <intent-filter>
                <action android:name="android.accessibilityservice.AccessibilityService" />
            </intent-filter>
            <meta-data
                android:name="android.accessibilityservice"
                android:resource="@xml/xmm_accessibility" />
        </service>
"""
    if "</application>" not in man:
        die("Manifest 里没找到 </application>")
    man = man.replace("</application>", service_xml + "    </application>")
    print("patch -> Manifest: <service>")

if "ShizukuProvider" not in man:
    provider_xml = """
        <!-- 咪的天：Shizuku（选填）。用户在 Shizuku App 里授权后，
             本应用可获得 shell 级身份执行命令。 -->
        <provider
            android:name="rikka.shizuku.ShizukuProvider"
            android:authorities="${applicationId}.shizuku"
            android:enabled="true"
            android:exported="true"
            android:multiprocess="false" />
"""
    man = man.replace("</application>", provider_xml + "    </application>")
    print("patch -> Manifest: <provider>")

# 网络权限（WebSocket 直连需要；Tauri 默认可能没有）
if "android.permission.INTERNET" not in man:
    man = man.replace(
        "<application",
        '    <uses-permission android:name="android.permission.INTERNET" />\n'
        '    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />\n'
        '    <uses-permission android:name="android.permission.ACCESS_WIFI_STATE" />\n\n'
        "    <application",
        1,
    )
    print("patch -> Manifest: INTERNET 权限")

# 录音 / 摄像头（语音消息、通话）
for perm in ("RECORD_AUDIO", "CAMERA", "MODIFY_AUDIO_SETTINGS"):
    tag = "android.permission." + perm
    if tag not in man:
        man = man.replace(
            "<application",
            '    <uses-permission android:name="%s" />\n\n    <application' % tag,
            1,
        )
        print("patch -> Manifest:", tag)

with open(manifest, "w", encoding="utf-8") as f:
    f.write(man)

# ---------------------------------------------------------------- 3. Gradle
with open(gradle, "r", encoding="utf-8") as f:
    gr = f.read()

if "dev.rikka.shizuku" not in gr:
    # 找到 dependencies { ... } 块，塞进去
    m = re.search(r"dependencies\s*\{", gr)
    if not m:
        die("build.gradle.kts 里没找到 dependencies 块")
    inject = (
        '\n    // 咪的天：Shizuku（选填的 shell 身份来源）\n'
        '    implementation("dev.rikka.shizuku:api:13.1.5")\n'
        '    implementation("dev.rikka.shizuku:provider:13.1.5")\n'
    )
    idx = m.end()
    gr = gr[:idx] + inject + gr[idx:]
    print("patch -> build.gradle.kts: Shizuku 依赖")

with open(gradle, "w", encoding="utf-8") as f:
    f.write(gr)

# ------------------------------------------------- 3b. SDK 版本 + 明文流量
# Tauri 生成时写的是 compileSdk=37，本机只装了 platform-35；
# 另外 release 默认 usesCleartextTraffic=false，而我们的 WebSocket 是 ws://
# 明文连接，必须放开，否则连不上。
with open(gradle, "r", encoding="utf-8") as f:
    gr2 = f.read()
orig = gr2
gr2 = re.sub(r"compileSdk\s*=\s*\d+", "compileSdk = 35", gr2)
gr2 = re.sub(r"targetSdk\s*=\s*\d+", "targetSdk = 35", gr2)
gr2 = gr2.replace(
    'manifestPlaceholders["usesCleartextTraffic"] = "false"',
    'manifestPlaceholders["usesCleartextTraffic"] = "true"',
)
if gr2 != orig:
    with open(gradle, "w", encoding="utf-8") as f:
        f.write(gr2)
    print("patch -> build.gradle.kts: compileSdk/targetSdk=35 + 允许 ws:// 明文")

# ------------------------------------------------- 3c. 降级 AGP / Kotlin / Gradle
# tauri android init 默认生成 AGP 9.3.1 + Kotlin 2.2.10 + Gradle 9.6.1。
# AGP 9 默认关掉 legacy variant API，Tauri 的 RustPlugin 依赖它，
# 直接报 "does not specify compileSdk" 和 "mergeUniversalDebugJniLibFolders not found"。
# 降到 Tauri 实际适配的组合。
AGP_VER = "8.7.3"
KOTLIN_VER = "2.0.21"
GRADLE_VER = "8.11.1"

_root_gradle = os.path.join(GEN, "build.gradle.kts")
_bs_gradle = os.path.join(GEN, "buildSrc", "build.gradle.kts")
_wrapper = os.path.join(GEN, "gradle", "wrapper", "gradle-wrapper.properties")

for p in (_root_gradle, _bs_gradle):
    if not os.path.isfile(p):
        continue
    src = open(p, encoding="utf-8").read()
    new = re.sub(r"com\.android\.tools\.build:gradle:[\d.]+", f"com.android.tools.build:gradle:{AGP_VER}", src)
    new = re.sub(r"kotlin-gradle-plugin:[\d.]+", f"kotlin-gradle-plugin:{KOTLIN_VER}", new)
    if new != src:
        open(p, "w", encoding="utf-8").write(new)
        print(f"patch -> {os.path.relpath(p, GEN)}: AGP {AGP_VER} / Kotlin {KOTLIN_VER}")

if os.path.isfile(_wrapper):
    src = open(_wrapper, encoding="utf-8").read()
    new = re.sub(r"gradle-[\d.]+-bin\.zip", f"gradle-{GRADLE_VER}-bin.zip", src)
    if new != src:
        open(_wrapper, "w", encoding="utf-8").write(new)
        print(f"patch -> gradle-wrapper.properties: Gradle {GRADLE_VER}")

# AGP 8 没有 optimization {} 这个 DSL
src = open(gradle, encoding="utf-8").read()
new = re.sub(
    r"\n\s*optimization\s*\{\s*\n\s*enable\s*=\s*\w+\s*\n\s*\}\n",
    "\n            isMinifyEnabled = false\n",
    src,
)
if new != src:
    open(gradle, "w", encoding="utf-8").write(new)
    print("patch -> build.gradle.kts: 移除 AGP9 的 optimization{} 块")

# ---------------------------------------------------------------- 4. strings
additions = {
    "xmm_a11y_label": "咪的天 协助服务",
    "xmm_a11y_desc": "允许「咪的天」在对方主动邀请并获你同意后，读取界面并代为操作。不开启也能正常聊天。",
}
if os.path.isfile(strings):
    with open(strings, "r", encoding="utf-8") as f:
        st = f.read()
    changed = False
    for k, v in additions.items():
        if 'name="%s"' % k not in st:
            st = st.replace("</resources>", '    <string name="%s">%s</string>\n</resources>' % (k, v))
            changed = True
    if changed:
        with open(strings, "w", encoding="utf-8") as f:
            f.write(st)
        print("patch -> strings.xml")

print("\n完成。下一步：npx tauri android build --apk")
