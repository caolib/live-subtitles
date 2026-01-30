# Tauri v2 完全透明无边框窗口配置

## 问题描述

在 Tauri v2 中，当设置窗口为透明 (`transparent: true`) 且无边框 (`decorations: false`) 时，在 Windows 平台上仍然会出现一个 **1px 的白色边框**。这是 WebView2 窗口的阴影效果导致的。

## 解决方案

在 `tauri.conf.json` 的窗口配置中添加 `"shadow": false`：

```json
{
  "app": {
    "windows": [
      {
        "label": "main",
        "title": "Live Subtitles",
        "width": 800,
        "height": 200,
        "decorations": false,
        "transparent": true,
        "shadow": false,
        "alwaysOnTop": true,
        "resizable": true,
        "visible": true
      }
    ]
  }
}
```

## 关键配置说明

| 属性 | 值 | 说明 |
|------|-----|------|
| `decorations` | `false` | 移除系统窗口边框和标题栏 |
| `transparent` | `true` | 启用窗口透明 |
| `shadow` | `false` | **关键！** 禁用窗口阴影，去除白色边框 |

## 平台特定行为

根据 [Tauri 官方文档](https://v2.tauri.app/reference/config/#windowconfig)：

> **shadow** (Windows 平台):
> - `false`: 对有边框的窗口无效，阴影始终开启
> - `true`: 会让无边框窗口有一个 **1px 白色边框**，在 Windows 11 上还会有圆角效果

因此，要实现完全透明无边框的效果，必须同时设置：
- `decorations: false`
- `transparent: true`  
- `shadow: false`

## 前端 CSS 配合

确保前端页面的背景也设置为透明：

```css
html, body {
  background: transparent;
}
```

如需半透明背景效果：

```css
.subtitle-container {
  background: rgba(0, 0, 0, 0.7);
}
```
