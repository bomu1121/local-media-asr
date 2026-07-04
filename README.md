# 本地媒体 ASR

> 纯本地运行的桌面端音视频转文字工具，基于离线语音识别模型，将视频/音频中的人声快速转换为可编辑的文本与字幕，全程本地运算保障数据隐私。

## 功能特性

- **一键暗箱处理**：选择文件 → 点击播放按钮，自动完成音频提取 + VAD 语音分段 + ASR 转写，全程一个进度条
- **双引擎切换**：内置快速引擎（SenseVoice-Small，230MB）和精准引擎（Paraformer-Large，231MB），按需切换
- **多格式输出**：支持 TXT / SRT / VTT / LRC / JSON 五种导出格式，一键复制或保存文件
- **智能语音分段**：基于 Silero-VAD 自动检测语音起止点，按语义停顿智能切分段落
- **历史记录**：转写结果自动存入 SQLite 本地数据库，支持历史回溯、复制、删除
- **离线隐私**：全部运算在本地完成，无需联网，数据不出设备

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | Tauri 2.0 |
| 前端 | Vue 3 + TypeScript + Naive UI |
| ASR 引擎 | sherpa-onnx (SenseVoice-Small / Paraformer-Large) |
| VAD | Silero-VAD (sherpa-onnx) |
| 音视频处理 | FFmpeg Sidecar |
| 本地存储 | SQLite (rusqlite) |
| 构建 | pnpm + Vite + Cargo (MSVC) |

## 快速开始

### 环境要求

- Windows 10+ (x64)
- [Rust](https://www.rust-lang.org/) (MSVC toolchain)
- [Node.js](https://nodejs.org/) 18+
- [pnpm](https://pnpm.io/)
- [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/) (C++ 桌面开发)
- [LLVM](https://github.com/llvm/llvm-project/releases) (clang, 用于 sherpa-rs-sys 编译)

### 安装依赖

```bash
pnpm install
```

### 准备模型

下载以下模型到 `models/` 目录：

| 模型 | 大小 | 用途 | 必需 |
|---|---|---|---|
| [SenseVoice-Small int8](https://www.modelscope.cn/models/iic/SenseVoiceSmall/int8) | 233MB | 快速转写引擎 | ✓ |
| [Paraformer-Large int8](https://www.modelscope.cn/models/iic/speech_paraformer-large-vad-punc_asr_nat-zh-cn-16k-common-vocab8404-pytorch) | 231MB | 精准引擎（含时间戳） | |
| [Silero-VAD](https://github.com/k2-fsa/sherpa-onnx/releases) | 1MB | 语音活动检测 | ✓ |
| [CT-Transformer Punct](https://www.modelscope.cn/models/iic/punc_ct-transformer_zh-cn-common-vocab272727-pytorch) | 67MB | 标点恢复 | |

模型目录结构：

```
models/
  sense-voice-small/    → 内含 model.int8.onnx, tokens.txt
  paraformer-large/     → 内含 model.int8.onnx, tokens.txt
  silero-vad/           → 内含 silero_vad.onnx
  punct-ct-transformer/ → 内含 model.onnx
```

### 准备 sherpa-onnx 动态库

下载 `sherpa-onnx-v1.12.9-win-x64-shared.tar.bz2`，解压到 `src-tauri/sherpa-onnx-lib/`。

### 开发运行

```bash
dev.bat
```

或手动设置环境变量后启动：

```bash
set SHERPA_LIB_PATH=src-tauri\sherpa-onnx-lib\sherpa-onnx-v1.12.9-win-x64-shared
set LIBCLANG_PATH=C:\Program Files\LLVM\bin
set CI=true
pnpm tauri dev
```

### 构建发布包

```bash
pnpm tauri build
```

## 项目结构

```
local-media-asr/
  src/                          # Vue 3 前端
    components/
      MainLayout.vue            # 三栏布局（导航 | 内容 | 结果）
      FileManager.vue           # 文件选择 + 一键处理 + 任务列表
      ResultPanel.vue           # 右侧结果面板（格式切换/复制/导出）
      TranscriptionPanel.vue    # 历史记录列表
      SettingsPanel.vue         # 引擎/VAD/导出设置
    stores/app.ts               # Pinia 全局状态
    utils/invoke.ts             # Tauri 命令封装
  src-tauri/                    # Rust 后端
    src/
      commands.rs               # Tauri 命令（process_media, export 等）
      pipeline.rs               # 转写流水线（WAV → VAD → ASR → 结果）
      asr.rs                    # ASR 引擎封装（双引擎切换）
      ffmpeg.rs                 # FFmpeg 音频提取
      db.rs                     # SQLite 历史记录 CRUD
      export.rs                 # 多格式导出（TXT/SRT/VTT/LRC/JSON）
    Cargo.toml
  models/                       # AI 模型文件（.onnx）
  dev.bat                       # 开发启动脚本
```

## 架构

```
前端 UI (Vue3 + NaiveUI)
    ↓ invoke/event (IPC)
Rust 核心服务
    ├── FFmpeg Sidecar (音频提取: 视频→16kHz mono WAV)
    ├── VAD (Silero-VAD: 语音端点检测与智能分段)
    ├── ASR 调度 (SenseVoice-Small / Paraformer-Large)
    ├── 后处理 (标点恢复、多格式生成)
    └── SQLite (历史记录持久化)
    ↓ FFI
sherpa-onnx C API DLL (v1.12.9)
    └── ONNX Runtime (推理引擎)
```

## License

MIT
