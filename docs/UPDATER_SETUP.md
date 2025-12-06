# Tauri 自动更新配置指南

本指南介绍如何为 Live Subtitles 配置自动更新功能。

## 概述

Tauri 自动更新器允许应用程序自动检测并安装新版本，无需用户手动下载。

## 工作原理

1. 应用启动时（或用户触发检查时）向 GitHub Releases 请求 `latest.json`
2. 比较当前版本与最新版本
3. 如果有新版本，显示更新对话框
4. 用户确认后，下载并验证更新包
5. 安装更新并重启应用

## 配置步骤

### 1. 生成签名密钥对

签名密钥用于验证更新包的真实性，防止恶意篡改。

```bash
cd src-tauri
pnpm tauri signer generate -- -w ~/.tauri/live-subtitles.key
```

输出示例：
```
Your keypair was generated successfully
Private: C:\Users\YourName\.tauri\live-subtitles.key (Keep this secret!)
Public: dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6...

Please keep your private key in a secure location and don't share it with anyone!
Your public key will be used to verify the signature of your updates.
```

⚠️ **重要**: 妥善保管私钥文件，不要泄露或提交到版本控制！

### 2. 配置公钥

将生成的公钥（以 `dW50cnVzdGVk` 开头的长字符串）复制到 `tauri.conf.json`：

```json
{
  "app": {
    "updater": {
      "active": true,
      "endpoints": [
        "https://github.com/caolib/live-subtitles/releases/latest/download/latest.json"
      ],
      "dialog": true,
      "pubkey": "YOUR_PUBLIC_KEY_HERE"
    }
  }
}
```

### 3. 配置 GitHub Secrets

在 GitHub 仓库设置中添加密钥：

1. 访问 `https://github.com/caolib/live-subtitles/settings/secrets/actions`
2. 点击 "New repository secret"
3. 添加以下 secrets：

**TAURI_SIGNING_PRIVATE_KEY**
- 值：私钥文件的完整内容
- 获取方式：
  ```powershell
  Get-Content ~\.tauri\live-subtitles.key -Raw
  ```

**TAURI_SIGNING_PRIVATE_KEY_PASSWORD** (可选)
- 值：生成密钥时设置的密码
- 如果没有设置密码，留空或不创建此 secret

### 4. 验证配置

提交更改并推送：

```bash
git add src-tauri/tauri.conf.json
git commit -m "feat: 启用自动更新功能"
git push
```

### 5. 测试更新流程

1. 使用 `release.ps1` 发布新版本
2. 等待 GitHub Actions 构建完成
3. 验证 Release 页面有以下文件：
   - `latest.json`
   - `*.msi.sig` / `*.exe.sig` 签名文件
4. 在旧版本应用中测试更新检测

## latest.json 文件结构

由 GitHub Actions 自动生成：

```json
{
  "version": "1.0.0",
  "notes": "查看完整更新日志...",
  "pub_date": "2024-01-01T00:00:00Z",
  "platforms": {
    "windows-x86_64": {
      "signature": "dW50cnVzdGVkIGNvbW1lbnQ6...",
      "url": "https://github.com/caolib/live-subtitles/releases/download/1.0.0/live-subtitles_1.0.0_x64-setup.nsis.zip"
    }
  }
}
```

## 应用内触发更新检查

已在"关于"对话框中实现：

```javascript
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

async function checkForUpdates() {
  const update = await check();
  if (update?.available) {
    const confirmed = window.confirm(
      `发现新版本 ${update.version}，是否立即更新？`
    );
    if (confirmed) {
      await update.downloadAndInstall();
      await relaunch();
    }
  }
}
```

## 更新器配置选项

### dialog 模式

```json
"updater": {
  "dialog": true  // 显示系统原生更新对话框
}
```

- ✅ 优点：无需自己实现 UI
- ❌ 缺点：样式无法自定义

### 手动模式

```json
"updater": {
  "dialog": false  // 使用自定义 UI
}
```

需要在代码中手动处理更新流程。

### endpoints 配置

支持多个更新源：

```json
"endpoints": [
  "https://github.com/caolib/live-subtitles/releases/latest/download/latest.json",
  "https://cdn.example.com/latest.json"  // 备用 CDN
]
```

更新器会按顺序尝试，直到成功。

## 安全注意事项

### 保护私钥

- ✅ 使用 GitHub Secrets 存储私钥
- ✅ 本地私钥文件使用文件系统权限保护
- ❌ 不要通过邮件、聊天工具分享私钥
- ❌ 不要在代码、配置文件或文档中包含私钥

### 验证更新

- 所有更新包都有签名文件（`.sig`）
- 更新器会自动验证签名
- 签名不匹配会拒绝安装

### HTTPS 传输

- GitHub Releases 强制使用 HTTPS
- 防止中间人攻击篡改更新包

## 故障排除

### 更新检查失败

1. 检查 `latest.json` URL 是否可访问
2. 验证公钥是否正确配置
3. 查看浏览器控制台错误信息

### 签名验证失败

```
Error: Failed to verify signature
```

- 原因：公钥与私钥不匹配
- 解决：重新生成密钥对，更新配置

### GitHub Actions 构建没有生成签名

- 检查 Secrets 是否正确配置
- 验证 workflow 文件中的环境变量

### 更新下载缓慢

- 考虑添加 CDN 镜像作为备用 endpoint
- 优化安装包大小

## 代理配置

### 方式一：使用自定义代理（推荐）

应用内置了自定义代理配置功能：

1. **打开设置界面** - 点击托盘图标 → 设置
2. **找到"网络设置"卡片**
3. **启用自定义代理开关**
4. **填写代理信息**：
   - **代理地址**: 例如 `http://proxy.company.com:8080`
   - **用户名**（可选）: 如果代理需要认证
   - **密码**（可选）: 如果代理需要认证

**支持的代理协议**：
- HTTP 代理：`http://host:port`
- HTTPS 代理：`https://host:port`

**配置示例**：

```
代理地址: http://proxy.example.com:8080
用户名: myuser
密码: mypass123
```

### 方式二：使用系统代理

如果不启用自定义代理，应用会**自动使用系统代理设置**。

**Windows 系统代理设置位置**：
1. 设置 → 网络和 Internet → 代理
2. 或使用 `控制面板 → Internet 选项 → 连接 → 局域网设置`

### 超时配置

代码中已配置 60 秒超时，适应代理环境：

```javascript
const config = {
    timeout: 60000, // 60秒超时
    proxy: {
        all: {
            url: 'http://proxy.example.com:8080',
            basicAuth: {
                username: 'user',
                password: 'pass'
            }
        }
    }
};
const update = await check(config);
```

### 代理环境测试

如果你在代理环境中：

1. **确认系统代理已配置**
   - Windows: 检查"Internet 选项"中的代理设置
   - 或在命令行中设置环境变量：
     ```powershell
     $env:HTTP_PROXY = "http://proxy.example.com:8080"
     $env:HTTPS_PROXY = "https://proxy.example.com:8080"
     ```

2. **测试更新检查**
   - 打开应用"关于"对话框
   - 点击"检查更新"
   - 查看是否能成功获取更新信息

3. **查看日志**
   - 如果失败，检查控制台错误信息
   - 超时错误通常表示代理配置问题

### 企业环境注意事项

如果你的企业环境使用**需要认证的代理**：

- Tauri updater 会尝试使用系统保存的凭据
- 如果失败，可能需要联系 IT 部门配置代理白名单
- 或考虑使用直连的更新镜像服务器

### 故障排除

**无法连接到更新服务器**：
1. 检查系统代理设置是否正确
2. 确认代理服务器允许访问 `github.com`
3. 尝试在浏览器中访问 `https://github.com/caolib/live-subtitles/releases/latest/download/latest.json`
4. 如果浏览器可以访问但应用不行，可能是防火墙阻止了应用

**下载速度慢**：
- 这是正常现象，取决于代理服务器和网络质量
- 60秒超时通常足够下载几MB的更新包

## 版本兼容性

- Tauri v2 使用新的更新器 API
- 配置文件路径：`tauri.conf.json` 的 `app.updater`
- 插件：`tauri-plugin-updater`

## 参考资源

- [Tauri 更新器官方文档](https://tauri.app/v1/guides/distribution/updater/)
- [Tauri 签名密钥生成](https://tauri.app/v1/guides/distribution/sign-windows/)
- [GitHub Actions Secrets 管理](https://docs.github.com/en/actions/security-guides/encrypted-secrets)

## 常见问题

### Q: 是否必须配置签名？

A: 强烈推荐。虽然技术上可以跳过签名（移除 pubkey 配置），但这会带来安全风险。

### Q: 可以使用自己的服务器吗？

A: 可以。只需将 `latest.json` 部署到你的服务器，并更新 endpoints URL。

### Q: 如何回滚到旧版本？

A: 更新器只支持向前更新。回滚需要用户手动卸载并安装旧版本。

### Q: 如何禁用自动更新？

A: 设置 `updater.active: false` 或完全移除 updater 配置块。

### Q: 更新频率如何控制？

A: 应用启动时自动检查，或通过"关于"对话框手动触发。不支持后台静默更新。
