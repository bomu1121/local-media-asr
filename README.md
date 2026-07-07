# 音视频转写助手

> 本地运行的桌面端音视频转文字工具。拖入视频/音频，自动提取人声、分段转录、AI校对，输出可直接使用的口播文案或字幕。全程本地运算保障数据隐私。

## 功能特性

- **一键转写**：拖入文件 → 自动音频提取 → Python Paraformer 分段 ASR → DeepSeek AI 校对 → 输出口播级文案
- **AI 智能校对**：转录完成后自动调用 DeepSeek API 修正同音错字、恢复专有名词、补全标点、划分段落，输出可直接发布的文案
- **双引擎支持**：快速引擎 (SenseVoice-Small) 和精准引擎 (Paraformer-Large)，Python 侧 Paraformer 提供更高准确率
- **多格式导出**：支持 TXT / SRT / VTT / LRC / JSON 五种格式，一键复制或保存
- **历史记录**：转写结果自动存入 SQLite 本地数据库，支持历史回溯
- **离线隐私**：核心计算在本地完成，AI 校对可选开启

## 快速开始

### 环境要求

- Windows 10+ (x64)
- Rust MSVC toolchain (`rustup default stable-x86_64-pc-windows-msvc`)
- Python 3.12+ （需安装 `sherpa-onnx`, `numpy`）
- FFmpeg（需在 PATH 中或通过应用内下载）
- pnpm（前端包管理）

### 安装依赖

```bash
# 前端
pnpm install

# Python (ASR worker)
pip install sherpa-onnx numpy

# Rust (MSVC toolchain)
rustup default stable-x86_64-pc-windows-msvc

# 以下仅打包时需要
pip install pyinstaller
```

### 下载/放置模型

将模型放置于项目根目录 `models/` 下，构建脚本会自动创建 junction 使 Tauri 能访问到：

```
models/
  paraformer-large/    # model.int8.onnx + tokens.txt
  sense-voice-small/   # model.int8.onnx
  silero-vad/          # silero_vad.onnx
  punct-ct-transformer/ # model.onnx
```

### 开发运行

```bash
pnpm tauri dev
# 或
.\dev.bat
```

### 生产打包

```bash
.\build.ps1
```

产物位置：

| 产物 | 路径 |
|------|------|
| NSIS 安装器 | `src-tauri/target/release/bundle/nsis/音视频转写助手_0.1.0_x64-setup.exe` |
| 主程序 | `src-tauri/target/release/media-transcriber.exe` |
| ASR 侧车 | `src-tauri/binaries/asr_worker.exe` |
| PyInstaller 临时文件 | `build/pyinstaller/` |

## 使用说明

1. 打开应用，点击左侧文件图标或拖入音视频文件
2. 在设置页配置 AI 校对（填入 DeepSeek API Key 并开启开关）
3. 选中文件，点击“开始提取”按钮开始转录
4. 等待 Python ASR 处理完成（9分钟视频约12秒）
5. 如开启 AI 校对，转录完成后自动调用 DeepSeek 修正文本
6. 在右侧面板切换格式查看结果，支持复制和导出

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | Tauri 2.0 |
| 前端 | Vue 3 + TypeScript + Naive UI |
| ASR 引擎 | sherpa-onnx (Python Paraformer-Large) |
| VAD | 能量检测 + 滑动窗口分段 |
| AI 校对 | DeepSeek API（可选，前端直连） |
| 音视频处理 | FFmpeg（系统安装或自动下载） |
| 本地存储 | SQLite (rusqlite) |
| 打包 | PyInstaller (ASR sidecar) + NSIS (安装器) |

## 项目结构

```
media-transcriber/
├── src/                    # Vue 前端
│   ├── components/         # Vue 组件
│   │   ├── FileManager.vue   # 文件管理 + 转录触发
│   │   ├── ResultPanel.vue   # 结果展示 + AI 校对
│   │   ├── SettingsPanel.vue  # 设置（引擎/API Key）
│   │   └── TranscriptionPanel.vue # 历史记录
│   ├── stores/app.ts       # Pinia 状态管理
│   └── utils/              # invoke / events / types
├── src-tauri/              # Rust 后端
│   ├── src/
│   │   ├── commands.rs     # Tauri commands
│   │   ├── ffmpeg.rs       # FFmpeg 集成
│   │   ├── export.rs       # 多格式导出
│   │   ├── db.rs           # SQLite 本地存储
│   │   └── lib.rs          # Tauri 入口
│   ├── asr_worker.py     # Python ASR worker（主力）
│   ├── binaries/         # sidecar 二进制（构建产物）
│   ├── models/            # ASR 模型文件
│   └── tauri.conf.json   # Tauri 配置
├── asr_worker.py           # ASR worker 副本（dev 模式用）
├── build.ps1               # 一键打包脚本
├── dev.bat                 # 开发启动脚本
└── models/                 # ASR 模型文件（不提交）
```

## 架构说明

ASR 通过 Python sidecar 进程完成，避免在 Rust 侧编译复杂的 sherpa-onnx FFI：

- **开发模式**：前端直接 `Command.create("python", ["src-tauri/asr_worker.py", ...])`
- **生产模式**：前端 `Command.sidecar("binaries/asr_worker", [...])`，调用 PyInstaller 打包的独立 exe

ASR worker 通过 stdout 输出 JSON lines 与前端通信，包含进度和最终结果段。
Tauri `tauri.conf.json` 的 `resources` 配置将模型打包进安装器，
`externalBin` 配置将 `asr_worker.exe` 作为 sidecar 管理。

## 常见问题

**Q: 转录结果碎片化？**
A: 检查设置中的 VAD 参数，或直接使用 Paraformer 引擎。Paraformer 配合 AI 校对可输出流畅长文案。

**Q: AI 校对没有生效？**
A: 确认设置页已填入 DeepSeek API Key 并开启开关。API Key 保存在浏览器 localStorage 中，刷新页面不会丢失。

**Q: 转写进度卡在提取音频阶段？**
A: Python ASR 处理期间显示“ASR 转写中”，等待 10-60 秒（取决于音频长度）后自动完成。

**Q: 打包时报错找不到 sidecar？**
A: 确认 `src-tauri/binaries/` 下有 `asr_worker-x86_64-pc-windows-msvc.exe`。用 `.\build.ps1` 打包可自动完成此步。
