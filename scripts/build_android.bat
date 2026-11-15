@echo off
REM XmmChat - Android release build (arm64 only)
REM The .so is cross-compiled by cargo beforehand and copied into jniLibs,
REM so the gradle rust tasks are skipped here (-x). They would otherwise
REM require `tauri android build` to be running (it serves build options over RPC).

set JAVA_HOME=F:\jdk21\jdk-21.0.12+8
set NODE_DIR=C:\Users\Administrator\.workbuddy\binaries\node\versions\22.22.2-6
set ANDROID_HOME=F:\AndroidSDK
set ANDROID_SDK_ROOT=F:\AndroidSDK
set NDK_HOME=F:\AndroidSDK\ndk\27.2.12479018
set GRADLE_USER_HOME=E:\gradle-home

set PATH=%NODE_DIR%;%JAVA_HOME%\bin;%PATH%

cd /d F:\XMMChat\src-tauri\gen\android

call F:\gradle-home\gradle-8.11.1\bin\gradle.bat assembleArm64Release --no-daemon --console=plain --project-cache-dir E:\gradle-project-cache -x rustBuildArm64Release -x rustBuildUniversalRelease
exit /b %ERRORLEVEL%
