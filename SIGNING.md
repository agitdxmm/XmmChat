# 签名说明

「咪的天」的 Windows 端和 Android 端都用同一个署名签名：**爱玩的小喵咪**。

> ⚠️ **本文档不含任何私钥或密码。** 密钥材料和口令都在 `%USERPROFILE%\.xmmchat\` 下，
> 与项目目录隔离。项目文件夹可以随便备份、分享、传给别人，不会因此泄露签名能力。

## 一、证书信息

| 项 | 值 |
|---|---|
| 主题（Subject） | `CN=爱玩的小喵咪, OU=咪的天, O=咪的天, C=CN` |
| 证书指纹（SHA-1） | `7E5108D106C4EB37F8E5D9828D06E0772BA84458` |
| 有效期 | 10000 天 |
| 类型 | 自签名（Self-signed） |

> 未填写 `L`（城市）与 `ST`（省份）—— 这两项是可选的，留空不影响签名有效性。

Android 端证书（APK 内）：

```
DN       : CN=爱玩的小喵咪, OU=咪的天, O=咪的天, C=CN
SHA-256  : d86d2b7af46882c3fa49f8dbf329e10a6f242241e4895fbcbd486ab5fe8c6635
```

## 二、密钥放在哪

```
%USERPROFILE%\.xmmchat\
├── xmmchat.keystore      Android 签名密钥（JKS，含私钥）
├── keystore.pass         keystore 口令（纯文本，28 位随机）
├── code-signing.cer      Windows 代码签名证书（公钥，可公开）
```

**这两个文件（keystore + pass）就是你的签名身份。** 谁能读到它们，谁就能签出冒充你的安装包。

建议：

1. 马上把 `keystore.pass` 的内容抄进密码管理器，然后删掉这个文件
2. 把 `xmmchat.keystore` 备份一份到加密网盘或 U 盘
3. 不要把它们放进 Git，也不要放进任何会分享出去的文件夹

`.gitignore` 里已经排除了 `*.keystore` / `*.jks` / `*.pfx` / `*.pass`。

## 三、已签名的文件

| 文件 | 大小 | 签名方式 |
|---|---|---|
| `release/xmmchat.exe` | 11 MB | Authenticode（SHA-256） |
| `release/咪的天_1.0.0_x64-setup.exe` | 7.5 MB | Authenticode（SHA-256） |
| `release/咪的天_1.0.0_arm64.apk` | 7.0 MB | APK Signature Scheme v1+v2+v3 |

`release/爱玩的小喵咪.cer` 是公钥证书，**可以**随安装包一起分发——它只能用来验签，不能用来签名。

## 四、验证签名

### Windows

```powershell
Get-AuthenticodeSignature "release\xmmchat.exe" |
    Format-List Status, SignerCertificate
```

### Android

```bash
F:\AndroidSDK\build-tools\35.0.0\apksigner.bat verify --print-certs release\咪的天_1.0.0_arm64.apk
```

## 五、让 Windows 认这个签名

自签名证书默认不在信任链里，`Status` 会显示 `UnknownError`（"证书链在不受信任的根证书中终止"）。
这不代表签名无效，只是系统不认识这个签发者。

要让 UAC 和 SmartScreen 显示「爱玩的小喵咪」：

1. 双击 `release/爱玩的小喵咪.cer`
2. 安装证书 → 存储位置选「**当前用户**」
3. 「将所有的证书都放入下列存储」→ 浏览 →「**受信任的根证书颁发机构**」
4. 完成，确认安全警告

装完 `Status` 会变成 `Valid`。这一步是可选的，不装也能正常安装运行。

## 六、重新签名

**Windows**（每次 `npm run app:build` 后签名都会丢，必须重签）：

```bash
powershell -ExecutionPolicy Bypass -File scripts\sign_windows.ps1
```

**Android**：

```powershell
$keydir = "$env:USERPROFILE\.xmmchat"
$pass = (Get-Content "$keydir\keystore.pass" -Raw).Trim()
$env:JAVA_HOME = "F:\jdk21\jdk-21.0.12+8"

& "F:\AndroidSDK\build-tools\35.0.0\apksigner.bat" sign `
  --ks "$keydir\xmmchat.keystore" --ks-pass "pass:$pass" `
  --ks-key-alias xmmchat --key-pass "pass:$pass" `
  --out "release\咪的天_1.0.0_arm64.apk" `
  "src-tauri\gen\android\app\build\outputs\apk\arm64\release\app-arm64-release-unsigned.apk"
```

## 七、换签名会导致什么

Android 的规则是：**同一应用升级必须使用同一签名**。

如果 keystore 丢了，新版本无法覆盖安装，只能：

1. 卸载旧版（聊天记录、协助授权全部清空）
2. 安装新版

所以那个 keystore 的价值等于"这个应用能不能继续升级"，别当普通文件对待。

## 八、应用内署名

设置 → 关于 → **作者 · 爱玩的小喵咪**
