<h1 align="center">
<img src=".\src-tauri\icons\128x128.png"/>
</h1>

<h1 align='center'>Live Subtitles - 实时字幕</h1>

<!-- Keep these links. Translations will automatically update with the README. -->
简体中文 |
[English](https://zdoc.app/en/caolib/live-subtitles) | 
[Deutsch](https://zdoc.app/de/caolib/live-subtitles) | 
[Español](https://zdoc.app/es/caolib/live-subtitles) | 
[français](https://zdoc.app/fr/caolib/live-subtitles) | 
[日本語](https://zdoc.app/ja/caolib/live-subtitles) | 
[한국어](https://zdoc.app/ko/caolib/live-subtitles) | 
[Português](https://zdoc.app/pt/caolib/live-subtitles) | 
[Русский](https://zdoc.app/ru/caolib/live-subtitles)

<p align="center">
  <img src="https://img.shields.io/github/downloads/caolib/live-subtitles/total?labelColor=grey&color=blue" alt="Downloads"/>
  <img src="https://img.shields.io/github/v/release/caolib/live-subtitles?labelColor=grey&color=red" alt="Release"/>
  <img src="https://img.shields.io/github/stars/caolib/live-subtitles" alt="Stars"/>
  <img src="https://img.shields.io/github/downloads/caolib/live-subtitles/latest/total" alt="Latest Downloads"/>
  <img src="https://img.shields.io/github/license/caolib/live-subtitles
  " alt="License"/>
</p>

一个实时字幕工具，基于 [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) 的 asr 模型完成实时字幕识别。

[](https://github.com/user-attachments/assets/40597985-8de4-4954-9a42-c47031b5e95a)

## 使用

下载安装应用，为了减小体积，应用本身没有捆绑模型，在应用的安装目录下有一个空的`models`文件夹，你可以将下载的模型放入该文件夹中。

模型下载：

- [预训练模型列表](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-transducer/index.html)

- [模型发布页面](https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models)

比如下载这个[中文语言模型](https://github.com/k2-fsa/sherpa-onnx/releases/download/asr-models/sherpa-onnx-streaming-zipformer-zh-xlarge-int8-2025-06-30.tar.bz2)，将模型文件解压到应用的`models`目录下，然后点击应用托盘图标打开设置界面，配置models目录即可自动识别，类似下面的目录结构

```
├── models
│   ├── sherpa-onnx-streaming-zipformer-zh-int8-2025-06-30
│   │   ├── decoder.onnx
│   │   ├── encoder.int8.onnx
│   │   ├── joiner.int8.onnx
│   │   ├── README.md
│   │   ├── test_wavs
│   │   │   ├── 0.wav
│   │   │   ├── 1.wav
│   │   │   └── 8k.wav
│   │   └── tokens.txt
```

![image-20251206174756043](https://s2.loli.net/2025/12/06/9b3MISCVzyOG5sk.png)

## 开发

前置条件：

- rust环境
- nodejs环境

克隆项目之后在根目录执行，使用你喜欢的包管理器即可

```bash
pnpm i
pnpm start
```

## 致谢

- [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) 语音识别模型支持
- [TMSpeech](https://github.com/jxlpzqc/TMSpeech) 参考实现

