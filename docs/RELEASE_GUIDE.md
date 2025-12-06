# 发布流程文档

## 快速发布

使用自动化脚本发布新版本：

```powershell
.\release.ps1
```

脚本会自动：
1. 显示当前版本并提供预设选项（补丁/次要/主要版本）
2. 更新 `tauri.conf.json`、`Cargo.toml` 和 `package.json` 中的版本号
3. 生成发布说明模板
4. 提交更改并推送到远程仓库
5. 创建并推送版本标签

## 完整流程

### 1. 准备发布

确保所有更改已提交并推送：

```bash
git status
git push
```

### 2. 运行发布脚本

```powershell
.\release.ps1
```

按照提示操作：
- 选择版本类型（补丁/次要/主要）或手动输入版本号
- 编辑 `docs/RELEASE.md` 填写更新内容
- 按 Enter 继续

### 3. 自动构建

推送标签后，GitHub Actions 会自动触发构建流程：

1. 访问 https://github.com/caolib/live-subtitles/actions
2. 查看 "发布应用" 工作流运行状态
3. 等待构建完成（约 10-15 分钟）

### 4. 验证发布

构建成功后：

1. 访问 https://github.com/caolib/live-subtitles/releases
2. 检查新版本是否已发布
3. 验证以下文件是否存在：
   - `live-subtitles_x.y.z_x64_zh-CN.msi` - MSI 安装包
   - `live-subtitles_x.y.z_x64_zh-CN.msi.sig` - 签名文件
   - `live-subtitles_x.y.z_x64-setup.exe` - EXE 安装包
   - `live-subtitles_x.y.z_x64-setup.exe.sig` - 签名文件
   - `latest.json` - 更新器配置文件

### 5. 测试自动更新

在旧版本应用中：
1. 打开"关于"对话框
2. 检查是否提示有新版本
3. 点击下载链接验证

## 版本号规范

遵循语义化版本（Semantic Versioning）：

- **主版本号 (Major)**: 不兼容的 API 更改
- **次版本号 (Minor)**: 向后兼容的新功能
- **补丁版本号 (Patch)**: 向后兼容的 bug 修复

示例：
- `0.1.0` → `0.1.1` (修复 bug)
- `0.1.1` → `0.2.0` (添加新功能)
- `0.2.0` → `1.0.0` (重大更新)

## 手动发布（备用方案）

如果自动脚本失败，可以手动执行：

### 1. 更新版本号

编辑以下文件：
- `src-tauri/tauri.conf.json`
- `src-tauri/Cargo.toml`
- `package.json`

### 2. 编写发布说明

创建 `docs/RELEASE.md`：

```markdown
## 版本 x.y.z

### 更新内容

- 添加了 XXX 功能
- 改进了 XXX 性能

### 修复

- 修复了 XXX 问题
```

### 3. 提交并推送

```bash
git add .
git commit -m "chore: 发布版本 x.y.z"
git push
```

### 4. 创建标签

```bash
git tag x.y.z
git push origin x.y.z
```

## 故障排除

### 构建失败

检查 GitHub Actions 日志：
1. 访问失败的工作流运行
2. 查看详细错误信息
3. 常见问题：
   - Rust 编译错误：检查 `Cargo.toml` 依赖
   - 前端构建错误：检查 `package.json` 和 Node 版本
   - 签名错误：验证 GitHub Secrets 配置

### 自动更新不工作

1. 检查 `tauri.conf.json` 中的 updater 配置
2. 验证 `latest.json` 文件是否正确生成
3. 确认签名密钥配置正确

### 标签推送失败

```powershell
# 删除本地标签
git tag -d x.y.z

# 删除远程标签
git push origin :refs/tags/x.y.z

# 重新创建
.\release.ps1
```

## GitHub Secrets 配置

为了使用自动更新功能，需要配置签名密钥：

### 1. 生成密钥对

```bash
# 在 src-tauri 目录运行
pnpm tauri signer generate -- -w ~/.tauri/myapp.key
```

这会生成：
- 私钥文件 `~/.tauri/myapp.key`
- 公钥（显示在控制台）

### 2. 配置 GitHub Secrets

在仓库设置中添加：

- `TAURI_SIGNING_PRIVATE_KEY`: 私钥文件内容
- `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`: 私钥密码（如果设置了）

### 3. 更新公钥

将公钥复制到 `tauri.conf.json` 的 `updater.pubkey` 字段。

## 注意事项

- ⚠️ **永远不要提交私钥到仓库**
- ✅ 每次发布前确保本地代码是最新的
- ✅ 发布前在本地测试构建：`pnpm tauri build`
- ✅ 发布说明要清晰明了，方便用户了解更新内容
- ✅ 重大更新建议先发布预览版（如 `1.0.0-beta.1`）

## 参考资源

- [Tauri 更新器文档](https://tauri.app/v1/guides/distribution/updater/)
- [语义化版本规范](https://semver.org/lang/zh-CN/)
- [GitHub Actions 文档](https://docs.github.com/en/actions)
