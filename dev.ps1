$env:CARGO_HOME = "$env:USERPROFILE\.cargo"
$env:RUSTUP_HOME = "$env:USERPROFILE\.rustup"
$env:Path = "$env:USERPROFILE\.cargo\bin;C:\Users\Administrator\.cache\codex-runtimes\codex-primary-runtime\dependencies\node\bin;C:\Users\Administrator\.cache\codex-runtimes\codex-primary-runtime\dependencies\bin;D:\Develop\local-media-asr\ffmpeg\ffmpeg-2026-07-02-git-95a888b9ca-essentials_build\bin;" + $env:Path
$env:CI = "true"
rustup default stable-x86_64-pc-windows-msvc 2>&1 | Out-Null
Set-Location D:\Develop\local-media-asr
pnpm tauri dev
