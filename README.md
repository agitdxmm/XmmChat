# 咪的天 · XmmChat

> 填个地址就能聊。点对点直连，不经过任何服务器。

一个 Windows + Android 双端的即时通信工具。核心只有一句话：**双方各自填对方 IP（或域名），在 19779 端口上直连**，中间不经过任何中转服务器。

界面按 **云母（Mica）→ 亚克力（Acrylic）→ 液态玻璃（Liquid Glass）** 三层材质搭起来，聊天气泡是 25px 毛玻璃 + 折射描边，背景可以换成你自己的图片。

---

## 一、它能做什么

| 能力 | 说明 |
|---|---|
| 文字消息 | Enter 发送、Shift+Enter 换行，带「对方正在输入」和已送达标记 |
| 任意文件 | 拖进窗口或点回形针，64 KB 分块传输，带实时进度条 |
| 语音消息 | 按住录音，实时电平波形，松手即发 |
| 语音 / 视频通话 | WebRTC 点对点，局域网不需要 STUN |
| 远程查看屏幕 | 被控方主动邀请后，以 ~2 fps 推送 PNG 画面 |
| 远程执行指令 | 被控方主动邀请后，对方的命令在本机 shell 中执行，输出回流到聊天 |
| 协助邀请 | **邀请制**：被控方发起，控制方接受，才建立能力 |
| 自定义外观 | 深/亮主题、背景图、模糊强度、气泡不透明度、消息密度 |

### 端口

固定 **19779**。两端都监听这个端口，谁先连谁当客户端，另一方自动接受。

地址支持：

```
192.168.1.20          局域网 IPv4
::1                   裸 IPv6（自动加方括号）
pc.local              局域网主机名
10.0.0.5:19779        自带端口
ws://a.example.com    完整 URL
```

---

## 二、远程能力的安全模型

这一块是刻意设计的，不是顺手加的。

```
   被控方                          控制方
     │                               │
     │  ① 点「邀请对方协助我」         │
     │  ② 勾选范围（看屏幕 / 执行指令） │
     │ ─────── assist-invite ───────► │
     │                               │  ③ 收到卡片
     │ ◄────── assist-reply ──────── │     点「同意」
     │  ④ 授予范围生效                 │
     │                               │
     │ ◄────── exec-req ──────────── │  ⑤ 此时才能执行
     │ ─────── exec-res ────────────► │
```

几个硬约束：

- **不能主动控制别人**。知道 IP 只能发消息，发不了命令。
- **授权是双向确认的**。邀请方发起、接收方同意，缺一不可。
- **授权随时可收回**。断链、退出程序、点「收回全部授权」，立即失效。
- **服务端二次校验**。即使前端被绕过，Rust 侧 `shell_run(granted=false)` 也会直接拒绝。

---

## 三、安卓端的权限

安卓应用默认没有 shell 身份。所以「远程执行指令」需要额外条件，**全部是选填**：

| 后端 | 怎么来 | 能干什么 |
|---|---|---|
| `shizuku` | 装 Shizuku App 并授权 | 拿到 shell 级 UID，可以执行命令 |
| `root` | 设备已 root | 同上 |
| `accessibility` | 开启无障碍服务 | 只能读界面 / 模拟操作，**不能跑 shell** |
| `none` | 都没有 | 显示「没有Shell权限的终端」 |

界面对应三种状态：

- 有 Shizuku / root → 终端解锁，可以输入命令
- 只有无障碍 → 终端显示 `无障碍已开启 · 无 shell`
- 什么都没有 → 显示 **权限不足**，终端下方小字 **「没有Shell权限的终端」**

聊天、文件、语音、通话**不依赖**这些权限，装完就能用。

开启入口在「设置 → 安卓权限」。

---

## 四、技术栈

```
┌──────────────────────────────────────────────┐
│  Vue 3 + TypeScript  (src/)                  │  一套前端，两端共用
│  ├─ 材质系统  styles/theme.css                │
│  ├─ 协议层    core/protocol.ts                │
│  ├─ 状态      core/store.ts                   │
│  ├─ 引擎      core/engine.ts                  │
│  └─ 通话      core/call.ts  (WebRTC)          │
├──────────────────────────────────────────────┤
│  Tauri 2 IPC  (invoke / event)                │  平台差异被吸收在这层
├──────────────────────────────────────────────┤
│  Windows: Rust                                │  Android: Rust + Kotlin
│  ├─ net.rs      WebSocket 服务端+客户端        │  ├─ 同左（Rust 通用）
│  ├─ shellx.rs   cmd /C 执行                   │  ├─ XmmShellPlugin.kt (Shizuku/root)
│  ├─ cap.rs      GDI 抓屏 → 手写 PNG            │  └─ XmmAccessibilityService.kt
│  └─ sysinfo.rs  主机名 / 局域网地址            │
└──────────────────────────────────────────────┘
```

为什么传输层用 Rust 而不是 JS：**Rust 代码在 Windows 和 Android 上是同一份**。Tauri 2 的移动端把同一套 Rust 编译成 `.so` 跑在安卓里，所以监听端口、收发帧、抓屏这些不需要在 Kotlin 里重写一遍。Kotlin 只负责 Rust 碰不到的东西（Shizuku 的 binder、无障碍服务）。

### 依赖

前端：`vue` `vite` `@tauri-apps/api`

Rust：`tauri` `tokio` `tokio-tungstenite` `futures-util` `serde` `base64` `local-ip-address` `flate2` `windows-sys`

安卓：`dev.rikka.shizuku:api` + `provider`（选填）

---

## 五、构建

### 环境

| 项 | 版本 | 备注 |
|---|---|---|
| Node | 22.x | |
| Rust | 1.99 | `x86_64-pc-windows-gnu` |
| JDK | 21 | 路径不能含中文 |
| Android SDK | platform 35 / build-tools 35 | |
| NDK | 27.2.12479018 | |

### Windows

```bash
npm install --ignore-scripts
npm run app:build
```

产物：

```
src-tauri/target/release/xiaomiaomi-zip.exe   → 实际是 xmmchat.exe
src-tauri/target/release/bundle/nsis/咪的天_1.0.0_x64-setup.exe
```

### Android

```bash
export ANDROID_HOME=/path/to/AndroidSDK
export NDK_HOME=$ANDROID_HOME/ndk/27.2.12479018
export JAVA_HOME=/path/to/jdk21

npx tauri android init
python _android_patch.py          # 打入无障碍 + Shizuku 补丁
npx tauri android build --apk
```

`_android_patch.py` 做的事：

1. 复制 `XmmShellPlugin.kt` / `XmmAccessibilityService.kt`
2. 复制 `res/xml/xmm_accessibility.xml`
3. Manifest 注入 `<service>` + Shizuku `<provider>` + INTERNET/录音/摄像头权限
4. `build.gradle.kts` 注入 Shizuku 依赖
5. `strings.xml` 注入无障碍说明文案

幂等，可以反复跑。

---

## 六、踩过的坑

**1. `decorations: false` 之后要自己画标题栏**

Windows 的默认标题栏和自绘界面拼在一起会非常割裂，所以窗口是无边框的，标题栏整条自己渲染：拖拽区用 `data-tauri-drag-region`，按钮调 `getCurrentWindow().minimize()` 等。capabilities 里必须放行 `core:window:allow-start-dragging` 等权限，否则点了没反应。

**2. `windowEffects.mica` 和 `decorations: false` 可以共存**

`tauri.conf.json` 里配 `"windowEffects": { "effects": ["mica"], "radius": 14 }`，无边框窗口依然能拿到 Win11 的云母底和圆角。但前端还是自己铺了一层渐变光斑，因为 mica 只在 Win11 22H2+ 生效，得给旧系统兜底。

**3. Rust 的 getter 不能在 `setup` 里同步调用**

`current_monitor()` / `outer_size()` 内部是「投递到事件循环 → 同步等结果」。`setup` 执行时事件循环还没跑，第一行就永久阻塞，表现是**进程活着、内存不动、一个窗口都不出来**。必须丢进独立线程、延迟几百毫秒再调。

**4. `cargo check` 卡住十几分钟没输出**

不是编译慢，是**卡在等 target 目录锁**。`tauri android init` 被中断后留下的 `cargo.exe` 子进程持有锁，新的 cargo 就一直在排队。杀进程立刻恢复。

**5. exFAT 不支持目录联接**

把 JDK 链接到英文路径时 `mklink /J` 报「函数不正确」—— 因为 F 盘是 exFAT。只能实打实复制一份。

**6. `indexmap` 的 sysroot 探测**

`schemars 0.8` 被开 `preserve_order` 后会走 `indexmap 1.9.3`，该 crate 的 `build.rs` 用 autocfg 探测 `std`，探测失败就不发 `has_std`，于是 `IndexMap<K,V,S>` 丢掉默认类型参数，编译直接失败。

对策：`Cargo.toml` 里显式写 `indexmap = { version = "1.9", features = ["std"] }`。如果 cargo 认为 build script「新鲜」不重跑，就得清掉对应的 `target/*/build/indexmap-*/output`。

---

## 七、已知限制

- **屏幕推送是截图流，不是视频流**。约 2 fps，画质够看清文字，不适合看视频。
- **跨公网需要一方有公网 IP 或做端口映射**。没有 STUN/TURN 时 WebRTC 通话也只能在局域网内。
- **文件传输走 base64 分块**，大文件（>500 MB）会吃内存，建议先压缩。
- **iOS 未适配**（Tauri 支持，但本项目没做）。

---

## 八、签名与致谢

所有成品都带有署名签名：

```
CN=爱玩的小喵咪, OU=咪的天, O=咪的天, L=北京, ST=北京, C=CN
```

Windows 用 Authenticode，Android 用 APK Signature Scheme v2+v3，证书与验签方法见 [SIGNING.md](SIGNING.md)。

图标由小喵咪手绘。材质系统参考 Windows 11 Fluent Design 与 iOS Liquid Glass 的公开设计规范。

界面里的 `没有Shell权限的终端` 这句话，是给安卓上没装 Shizuku 的人看的——它诚实地说明了一件事：**这个终端现在确实跑不了命令**。
