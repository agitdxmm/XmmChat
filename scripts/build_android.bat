@echo off
REM XmmChat - Android release build (arm64 only)
REM
REM 这个脚本跳过 gradle 的 rustBuild* 任务，因为它们的实现要求
REM `tauri android build` 在跑（它通过 RPC 提供构建参数）。
REM 所以下面两件事必须手动做：
REM   1. 把 cargo 编译好的 .so 放进 jniLibs
REM   2. 把 dist 的前端产物放进 assets
REM 漏掉第 2 步的症状：应用启动后去请求 http://localhost:1420 并返回 403。

set JAVA_HOME=F:\jdk21\jdk-21.0.12+8
set NODE_DIR=C:\Users\Administrator\.workbuddy\binaries\node\versions\22.22.2-6
set ANDROID_HOME=F:\AndroidSDK
set ANDROID_SDK_ROOT=F:\AndroidSDK
set NDK_HOME=F:\AndroidSDK\ndk\27.2.12479018
set GRADLE_USER_HOME=E:\gradle-home

set PATH=%NODE_DIR%;%JAVA_HOME%\bin;%PATH%

set ASSETS=F:\XMMChat\src-tauri\gen\android\app\src\main\assets
set JNILIBS=F:\XMMChat\src-tauri\gen\android\app\src\main\jniLibs\arm64-v8a

REM --- 1. 前端资源 ---
if not exist "F:\XMMChat\dist\index.html" (
    echo [ERROR] dist\index.html 不存在，先跑 npm run build
    exit /b 1
)
if exist "%ASSETS%\assets" rmdir /S /Q "%ASSETS%\assets"
if exist "%ASSETS%\index.html" del /Q "%ASSETS%\index.html"
xcopy /E /Y /Q "F:\XMMChat\dist\*" "%ASSETS%\" >nul
echo [assets] 已同步前端资源

REM --- 2. 原生库 ---
if not exist "%JNILIBS%\libxmmchat_lib.so" (
    echo [ERROR] jniLibs 里没有 libxmmchat_lib.so，先交叉编译
    exit /b 1
)
echo [jniLibs] libxmmchat_lib.so 就位

REM --- 3. 打包 ---
cd /d F:\XMMChat\src-tauri\gen\android
call F:\gradle-home\gradle-8.11.1\bin\gradle.bat assembleArm64Release --no-daemon --console=plain --project-cache-dir E:\gradle-project-cache -x rustBuildArm64Release -x rustBuildUniversalRelease
exit /b %ERRORLEVEL%
