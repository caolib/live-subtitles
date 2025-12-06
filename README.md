# Live Subtitles

一个实时字幕工具，基于 [sherpa-onnx](https://github.com/k2-fsa/sherpa-onnx) 的 asr 模型完成实时字幕识别。

[](https://github.com/user-attachments/assets/40597985-8de4-4954-9a42-c47031b5e95a)

## 使用

下载安装应用，为了减小体积，应用本身没有捆绑模型，在应用的安装目录下有一个空的models文件夹，你可以将下载的模型放入该文件夹中。

下载链接：

- [预训练模型列表](https://k2-fsa.github.io/sherpa/onnx/pretrained_models/online-transducer/index.html)

- [模型发布页面](https://github.com/k2-fsa/sherpa-onnx/releases/tag/asr-models)

将模型文件解压到应用的models目录下，然后点击应用托盘图标打开设置界面，配置models目录即可自动识别

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

