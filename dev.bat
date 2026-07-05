@echo off
set CARGO_HOME=%USERPROFILE%\.cargo
set RUSTUP_HOME=%USERPROFILE%\.rustup
set CI=true
set LIBCLANG_PATH=C:\Program Files\LLVM\bin
set CMAKE_Fortran_COMPILER=echo
set CMAKE_DISABLE_FIND_PACKAGE_Fortran=TRUE
set SHERPA_LIB_PATH=D:\Develop\local-media-asr\src-tauri\sherpa-onnx-lib\sherpa-onnx-v1.12.9-win-x64-shared
set PATH=%USERPROFILE%\.cargo\bin;%PATH%
rustup default stable-x86_64-pc-windows-msvc >nul 2>&1
cd /d D:\Develop\local-media-asr
pnpm tauri dev
pause