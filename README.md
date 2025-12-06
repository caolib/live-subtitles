<h1 align='center'>Live Subtitles - 实时字幕</h1>

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

## 发布

使用自动化脚本发布新版本：

```powershell
.\release.ps1
```

详细发布流程请参考 [发布指南](docs/RELEASE_GUIDE.md)。

### 自动更新配置

应用支持自动检查更新功能。首次发布需要配置签名密钥：

1. 生成密钥对：
   ```bash
   cd src-tauri
   pnpm tauri signer generate -- -w ~/.tauri/live-subtitles.key
   ```

2. 配置 GitHub Secrets（仓库设置 → Secrets）：
   - `TAURI_SIGNING_PRIVATE_KEY`: 私钥文件内容
   - `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: 私钥密码（如有）

3. 更新 `tauri.conf.json` 中的公钥

详细配置步骤请参考 [更新器配置指南](docs/UPDATER_SETUP.md)。

## 致谢

- [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) 语音识别模型支持
- [TMSpeech](https://github.com/jxlpzqc/TMSpeech) 参考实现

