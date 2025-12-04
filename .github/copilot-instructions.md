# Copilot Instructions for live-subtitles

## Project Overview

基于 Tauri v2 的实时字幕桌面应用，类似 [TMSpeech](https://github.com/jxlpzqc/TMSpeech)。

- **前端**: Vue 3 + Rsbuild
- **后端**: Rust + [sherpa-rs](https://github.com/thewh1teagle/sherpa-rs) (sherpa-onnx 的 Rust 绑定)
- **核心功能**: 捕获系统音频 → VAD 语音检测 → 流式 ASR 语音识别 → 实时字幕浮窗

## Architecture

```
src/                        # Vue 3 前端
  ├── main.js               # Vue 应用入口
  ├── App.vue               # 根组件 (字幕浮窗 UI)
  └── components/           # 组件 (字幕显示、设置面板等)

src-tauri/                  # Rust 后端
  ├── src/
  │   ├── lib.rs            # Tauri 命令和应用初始化
  │   ├── main.rs           # 程序入口
  │   ├── audio/            # 音频捕获模块 (WASAPI loopback)
  │   ├── asr/              # 语音识别模块 (sherpa-rs)
  │   └── vad/              # VAD 语音活动检测
  ├── models/               # sherpa-onnx 模型文件 (从 TMSpeech 复制)
  ├── capabilities/         # Tauri v2 权限配置
  └── tauri.conf.json       # Tauri 配置
```

## Key Commands

```bash
pnpm tauri dev      # 启动开发服务器
pnpm tauri build    # 构建生产版本
pnpm dev            # 仅前端开发 (localhost:1420)
```

## Core Implementation Pattern

### 1. 系统音频捕获 (Windows WASAPI Loopback)

使用 `cpal` 或 Windows WASAPI 捕获系统音频：

```rust
// src-tauri/src/audio/capture.rs
// 采样率必须为 16kHz (sherpa-onnx 要求)
```

### 2. 语音识别 (sherpa-rs)

```rust
// Cargo.toml 依赖
sherpa-rs = { version = "0.1", features = ["tts"] }

// 流式识别示例
use sherpa_rs::silero_vad::{SileroVad, SileroVadConfig};
use sherpa_rs::transducer::{TransducerConfig, TransducerRecognizer};

// VAD 配置 (512 samples window @ 16kHz)
let vad_config = SileroVadConfig {
    model: "models/silero_vad.onnx".into(),
    window_size: 512,
    ..Default::default()
};

// ASR 配置 (使用 TMSpeech 下载的模型)
let asr_config = TransducerConfig {
    encoder: "models/encoder.onnx".into(),
    decoder: "models/decoder.onnx".into(),
    joiner: "models/joiner.onnx".into(),
    tokens: "models/tokens.txt".into(),
    ..Default::default()
};
```

### 3. 前后端通信

```rust
// Rust 端 - 使用事件推送实时字幕
#[tauri::command]
fn start_recognition(window: tauri::Window) {
    // 在后台线程中运行识别
    std::thread::spawn(move || {
        // 识别到文字时发送事件
        window.emit("subtitle", payload).unwrap();
    });
}
```

```vue
<!-- Vue 端 - 监听字幕事件 -->
<script setup>
import { listen } from "@tauri-apps/api/event";

onMounted(async () => {
  await listen("subtitle", (event) => {
    subtitleText.value = event.payload;
  });
});
</script>
```

## Model Files (from TMSpeech)

模型文件放置在 `src-tauri/models/` 目录：

```
models/
  ├── silero_vad.onnx           # VAD 模型
  ├── encoder-*.onnx            # ASR encoder
  ├── decoder-*.onnx            # ASR decoder
  ├── joiner-*.onnx             # ASR joiner (transducer)
  └── tokens.txt                # 词表
```

## Tauri v2 Permissions

添加音频相关权限到 `src-tauri/capabilities/default.json`：

```json
{
  "permissions": ["core:default", "opener:default", "shell:allow-open"]
}
```

## UI Conventions

- 浮窗样式：无边框、可拖拽、置顶显示、透明背景
- 字幕显示：类似歌词滚动效果，支持历史记录
- 使用 `<script setup>` Composition API

## Build Notes

- 前端使用 Rsbuild (非 Vite)，配置文件: `rsbuild.config.js`
- 开发端口固定 1420
- sherpa-rs 需要编译 C++ 依赖，首次构建较慢
