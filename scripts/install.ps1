# Installer hakonfig-cli untuk Windows (PowerShell).
#
# Pakai:
#   irm https://raw.githubusercontent.com/perogeremmer/hakonfig/master/scripts/install.ps1 | iex
#
# Atau dengan versi sendiri:
#   install.ps1 v0.4.0
#
# Binary diunduh dari GitHub releases dan diverifikasi terhadap SHA256SUMS.
$ErrorActionPreference = "Stop"

$Repo = "perogeremmer/hakonfig"
$Bin = "hakonfig-cli.exe"
$Asset = "hakonfig-cli-x86_64-pc-windows-msvc.exe"

$Version = $args[0]
if ($Version) {
    $Base = "https://github.com/$Repo/releases/download/$Version"
} else {
    # Mengikuti rilis terakhir yang sudah di-publish (bukan draft).
    $Base = "https://github.com/$Repo/releases/latest/download"
}

$Dest = if ($env:HAKONFIG_INSTALL_DIR) {
    $env:HAKONFIG_INSTALL_DIR
} else {
    "$env:LOCALAPPDATA\Programs\hakonfig"
}

$Tmp = New-Item -ItemType Directory -Path (Join-Path $env:TEMP "hakonfig-install-$(Get-Random)") -Force

Write-Host "mengunduh $Asset ..."
Invoke-WebRequest -Uri "$Base/$Asset" -OutFile (Join-Path $Tmp $Bin)
Invoke-WebRequest -Uri "$Base/SHA256SUMS" -OutFile (Join-Path $Tmp "SHA256SUMS")

# ---------------------------------------------------------------- verifikasi
$Line = Select-String -Path (Join-Path $Tmp "SHA256SUMS") -Pattern [regex]::Escape($Asset) |
    Select-Object -First 1
if (-not $Line) {
    throw "checksum untuk $Asset tidak ada di SHA256SUMS"
}
$Expected = ($Line.Line -split '\s+')[0].ToLower()
$Actual = (Get-FileHash -Algorithm SHA256 (Join-Path $Tmp $Bin)).Hash.ToLower()
if ($Expected -ne $Actual) {
    throw "checksum tidak cocok: expected $Expected, got $Actual"
}
Write-Host "checksum ok"

# ---------------------------------------------------------------- pasang
New-Item -ItemType Directory -Path $Dest -Force | Out-Null
Move-Item -Force (Join-Path $Tmp $Bin) (Join-Path $Dest $Bin)
Remove-Item -Recurse -Force $Tmp

if ($env:PATH -notlike "*$Dest*") {
    Write-Host ""
    Write-Host "CATATAN: $Dest belum ada di PATH. Tambahkan dengan:"
    Write-Host "  [Environment]::SetEnvironmentVariable('Path', `$env:Path + ';$Dest', 'User')"
}

Write-Host "terpasang: $(Join-Path $Dest $Bin)"
