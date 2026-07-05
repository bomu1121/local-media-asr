# 本地媒体 ASR

> 纯本地运行的桌面端音视频转文字工具。拖入视频/音频，自动提取人声、分段转录、AI 校对，输出可直接使用的口播文案或字幕。全程本地运算保障数据隐私。

## 功能特性

- **一键转录**：拖入文件 → 自动音频提取 → Python Paraformer 分段 ASR → DeepSeek AI 校对 → 输出口播级文案
- **AI 智能校对**：转录完成后自动调用 DeepSeek API 修正同音错字、恢复专有名词、补全标点、划分段落，输出可直接发布的文案
- **双引擎支持**：快速引擎（SenseVoice-Small）和精准引擎（Paraformer-Large），Python 侧 Paraformer 提供更高准确率
- **多格式导出**：支持 TXT / SRT / VTT / LRC / JSON 五种格式，一键复制或保存
- **历史记录**：转写结果自动存入 SQLite 本地数据库，支持历史回溯
- **离线隐私**：核心计算在本地完成，AI 校对可选开启

## 技术栈

| 层 | 技术 |
|---|---|
| 桌面框架 | Tauri 2.0 |
| 前端 | Vue 3 + TypeScript + Naive UI |
| ASR 引擎 | sherpa-onnx (Python Paraformer-Large / Rust SenseVoice-Small) |
| VAD | 能量检测 + 滑动窗口分段 |
| AI 校对 | DeepSeek API（可选，前端直连） |
| 音视频处理 | FFmpeg（系统安装或自动下载） |
| 本地存储 | SQLite (rusqlite) |
| 构建 | pnpm + Vite + Cargo (MSVC) |

## 快速开始

### 环境要求

- Windows 10+ (x64)
- Python 3.12+（需安装 `sherpa-onnx`, `numpy`）
- FFmpeg（需在 PATH 中或通过应用内下载）

### 安装依赖

```bash
# 前端
pnpm install

# Python（ASR worker）
pip install sherpa-onnx numpy
```

### 下载模型

将 SenseVoice-Small 或 Paraformer-Large 模型放置于 `src-tauri/models/` 目录。

### 开发运行

```bash
pnpm tauri dev
```

或双击 `dev.bat`。

## 使用说明

1. 打开应用，点击左侧文件图标或拖入音视频文件
2. 在设置页配置 AI 校对（填入 DeepSeek API Key 并开启开关）
3. 选中文件，点击播放按钮开始转录
4. 等待 Python ASR 处理完成（9分钟视频约12秒）
5. 如开启 AI 校对，转录完成后自动调用 DeepSeek 修正文本
6. 在右侧面板切换格式查看结果，支持复制和导出

## 项目结构

```
local-media-asr/
├── src/                    # Vue 前端
│   ├── components/         # Vue 组件
│   │   ├── FileManager.vue # 文件管理 + 转录触发
│   │   ├── ResultPanel.vue # 结果展示 + AI 校对
│   │   └── SettingsPanel.vue # 设置（引擎/API Key）
│   ├── stores/app.ts       # Pinia 状态管理（含 localStorage 持久化）
│   └── utils/              # invoke / events / types
├── src-tauri/              # Rust 后端
│   └── src/
│       ├── commands.rs     # Tauri commands
│       ├── pipeline.rs     # 转录流水线
│       ├── vad.rs          # VAD 模块
│       ├── ffmpeg.rs       # FFmpeg 集成
│       └── refine.rs       # DeepSeek API（后端备选）
├── asr_worker.py           # Python ASR worker（主力）
└── models/                 # ASR 模型文件
```

## 常见问题

**Q: 转录结果碎片化？**  
A: 检查设置中的 VAD 参数，或直接使用 Paraformer 引擎。Paraformer 配合 AI 校对可输出流畅长文案。

**Q: AI 校对没有生效？**  
A: 确认设置页已填入 DeepSeek API Key 并开启开关。API Key 保存在浏览器 localStorage 中，刷新页面不会丢失。

**Q: 转写进度卡在 30%？**  
A: Python ASR 处理期间进度条跳变不明显，等待 10-60 秒（取决于音频长度）后会自动完成。
