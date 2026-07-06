#!/usr/bin/env pwsh
# Build script for local-media-asr
# Produces NSIS installer at: src-tauri\target\release\bundle\nsis\

$ErrorActionPreference = "Stop"
$ScriptDir = Split-Path -Parent $MyInvocation.MyCommand.Path
Set-Location $ScriptDir

Write-Host "=== Step 1/5: Checking Rust environment ===" -ForegroundColor Cyan
$env:CARGO_HOME = "$env:USERPROFILE\.cargo"
$env:RUSTUP_HOME = "$env:USERPROFILE\.rustup"
$env:PATH = "$env:USERPROFILE\.cargo\bin;$env:PATH"
$env:CI = "true"

try { rustc --version } catch { throw "Rust not found. Install Rust from https://rustup.rs" }
try { pnpm --version } catch { throw "pnpm not found. Run: npm install -g pnpm" }

Write-Host "`n=== Step 2/5: Installing Python dependencies ===" -ForegroundColor Cyan
pip install pyinstaller sherpa-onnx numpy --quiet

Write-Host "`n=== Step 3/5: Building ASR sidecar (PyInstaller) ===" -ForegroundColor Cyan
if (-not (Test-Path src-tauri\binaries)) { New-Item -ItemType Directory -Path src-tauri\binaries | Out-Null }

pyinstaller --onefile --noconsole --name asr_worker `
  --distpath src-tauri/binaries `
  --workpath build/pyinstaller `
  --specpath build/pyinstaller `
  --clean `
  --hidden-import sherpa_onnx `
  --hidden-import numpy `
  src-tauri\asr_worker.py

if (-not (Test-Path src-tauri\binaries\asr_worker.exe)) {
  throw "PyInstaller failed: asr_worker.exe not created"
}

# Tauri expects target-triple suffix for sidecar binaries
Copy-Item src-tauri\binaries\asr_worker.exe src-tauri\binaries\asr_worker-x86_64-pc-windows-msvc.exe -Force
Write-Host "  Sidecar built: $((Get-Item src-tauri\binaries\asr_worker.exe).Length / 1MB) MB" -ForegroundColor Green

Write-Host "`n=== Step 4/5: Ensuring models are available ===" -ForegroundColor Cyan
if (-not (Test-Path src-tauri\models)) {
  if (Test-Path models) {
    cmd /c "mklink /J src-tauri\models $ScriptDir\models"
    Write-Host "  Junction created: src-tauri/models -> models/" -ForegroundColor Green
  } else {
    Write-Host "  WARNING: models/ directory not found. The installer will lack model files." -ForegroundColor Yellow
  }
} else {
  Write-Host "  models already present in src-tauri/" -ForegroundColor Green
}

Write-Host "`n=== Step 5/5: Building Tauri app (NSIS installer) ===" -ForegroundColor Cyan
pnpm tauri build

$installer = Get-ChildItem src-tauri\target\release\bundle\nsis\*.exe | Select-Object -First 1
if ($installer) {
  Write-Host "`nDone! Installer: $($installer.FullName)" -ForegroundColor Green
  Write-Host "  Size: $([math]::Round($installer.Length / 1MB, 1)) MB" -ForegroundColor Green
} else {
  Write-Host "`nWARNING: No installer found. Check build output above." -ForegroundColor Yellow
}
