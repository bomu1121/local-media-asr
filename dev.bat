@echo off
set CARGO_HOME=%USERPROFILE%\.cargo
set RUSTUP_HOME=%USERPROFILE%\.rustup
set CI=true
set LIBCLANG_PATH=C:\Program Files\LLVM\bin
set PATH=%USERPROFILE%\.cargo\bin;%PATH%

rem Add pnpm to PATH (from Codex runtime)
set PNPM_PATH=C:\Users\Administrator\.cache\codex-runtimes\codex-primary-runtime\dependencies\bin
if exist "%PNPM_PATH%\pnpm.cmd" set PATH=%PNPM_PATH%;%PATH%

rustup default stable-x86_64-pc-windows-msvc >nul 2>&1
cd /d D:\Develop\media-transcriber
pnpm tauri dev
pause
