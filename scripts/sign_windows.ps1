# Re-sign the Windows binaries with the local self-signed code signing certificate.
#
# The certificate lives in Cert:\CurrentUser\My and holds its own private key,
# so no password is needed here. Nothing secret is stored in this script.
#
# Usage: powershell -ExecutionPolicy Bypass -File scripts\sign_windows.ps1

$ErrorActionPreference = "Stop"

$cert = Get-ChildItem Cert:\CurrentUser\My |
    Where-Object { $_.HasPrivateKey -and $_.Subject -like "*C=CN*" } |
    Sort-Object NotAfter -Descending |
    Select-Object -First 1

if (-not $cert) {
    throw "No code-signing certificate with a private key found in Cert:\CurrentUser\My"
}

Write-Output "Cert   : $($cert.Subject)"
Write-Output "Thumb  : $($cert.Thumbprint)"
Write-Output ""

$targets = @(
    "F:\XMMChat\src-tauri\target\release\xmmchat.exe",
    "F:\XMMChat\src-tauri\target\release\bundle\nsis\咪的天_1.0.0_x64-setup.exe"
)

foreach ($f in $targets) {
    if (Test-Path $f) {
        $r = Set-AuthenticodeSignature -FilePath $f -Certificate $cert
        Write-Output ("signed {0} -> {1}" -f (Split-Path $f -Leaf), $r.Status)
    } else {
        Write-Output ("missing: {0}" -f $f)
    }
}

# Refresh the copies under release/
$rel = "F:\XMMChat\release"
if (Test-Path $rel) {
    Copy-Item "F:\XMMChat\src-tauri\target\release\xmmchat.exe" $rel -Force
    Copy-Item "F:\XMMChat\src-tauri\target\release\bundle\nsis\咪的天_1.0.0_x64-setup.exe" $rel -Force
    Write-Output "release/ refreshed"
}
