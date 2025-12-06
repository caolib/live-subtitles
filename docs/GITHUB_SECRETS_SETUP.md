# GitHub Secrets 配置清单

## 必需的 Secrets

为了使 GitHub Actions 自动构建和签名工作，需要在仓库中配置以下 secrets。

### 访问 Secrets 设置

1. 访问仓库页面
2. 点击 Settings (设置)
3. 左侧菜单选择 Secrets and variables → Actions
4. 点击 New repository secret

## 配置列表

### 1. TAURI_SIGNING_PRIVATE_KEY ⭐ 必需

**用途**: 签名更新包，确保自动更新的安全性

**获取方式**:

1. 生成密钥对：
   ```bash
   cd src-tauri
   pnpm tauri signer generate -- -w ~/.tauri/live-subtitles.key
   ```

2. 复制私钥内容：
   ```powershell
   Get-Content ~\.tauri\live-subtitles.key -Raw | Set-Clipboard
   ```

3. 粘贴到 GitHub Secret

**重要提示**:
- ⚠️ 这是私钥，绝对不能泄露！
- ⚠️ 不要提交到版本控制
- ⚠️ 妥善保管本地私钥文件
- ✅ 建议备份到安全的地方（如密码管理器）

---

### 2. TAURI_SIGNING_PRIVATE_KEY_PASSWORD (可选)

**用途**: 私钥密码（如果生成时设置了密码）

**配置方式**:
- 如果生成密钥时**设置了密码**，填入该密码
- 如果生成密钥时**没有密码**，不需要创建此 secret

---

## 配置验证

### 检查清单

- [ ] TAURI_SIGNING_PRIVATE_KEY 已配置
- [ ] 密码（如有）已配置
- [ ] 公钥已更新到 `tauri.conf.json`
- [ ] 本地私钥文件已备份
- [ ] 本地私钥文件已添加到 `.gitignore`

### 测试配置

1. 提交并推送代码
2. 创建测试标签：
   ```bash
   git tag test-0.0.1
   git push origin test-0.0.1
   ```
3. 访问 Actions 页面查看构建
4. 构建成功后检查 Release 页面

成功标志：
- ✅ 生成了 `.sig` 签名文件
- ✅ 生成了 `latest.json` 文件
- ✅ 没有签名相关的错误

### 清理测试标签

```bash
git tag -d test-0.0.1
git push origin :refs/tags/test-0.0.1
```

在 GitHub Release 页面删除测试发布。

---

## 常见问题

### Q: 如何查看配置的 Secrets？

A: GitHub 不允许查看 secret 的值（安全机制）。只能看到名称和最后更新时间。如需修改，重新配置即可。

### Q: 可以用同一个密钥对多个项目吗？

A: 技术上可以，但不推荐。每个项目应该有独立的密钥对。

### Q: 密钥丢失怎么办？

A: 重新生成密钥对，更新所有配置。旧版本应用将无法使用自动更新（需要手动安装新版本）。

### Q: 如何轮换密钥？

A: 
1. 生成新密钥对
2. 更新 GitHub Secrets
3. 更新 `tauri.conf.json` 中的公钥
4. 发布新版本

旧版本应用首次更新时会接受新密钥。

### Q: 不配置签名会怎样？

A: 自动更新功能将无法使用。但手动下载安装不受影响。

---

## 安全最佳实践

### ✅ 应该做的

- 使用强密码保护私钥
- 定期备份私钥（加密存储）
- 限制私钥文件的访问权限
- 使用 GitHub Secrets 存储敏感信息
- 定期审查 Actions 日志

### ❌ 不应该做的

- 将私钥提交到版本控制
- 通过邮件/聊天工具分享私钥
- 在代码或配置文件中硬编码私钥
- 使用相同的密钥对多个项目
- 在公开的地方讨论密钥内容

---

## 其他 Secrets（未来可能需要）

### GITHUB_TOKEN

**说明**: GitHub Actions 自动提供，无需手动配置

**权限**: 用于创建 Release、上传资产

---

### 自定义 CDN 配置（可选）

如果未来需要将更新包部署到 CDN：

- `CDN_ACCESS_KEY`: CDN 访问密钥
- `CDN_SECRET_KEY`: CDN 密钥
- `CDN_ENDPOINT`: CDN 端点地址

---

## 参考资源

- [GitHub Actions Secrets 文档](https://docs.github.com/en/actions/security-guides/encrypted-secrets)
- [Tauri 签名指南](https://tauri.app/v1/guides/distribution/sign-windows/)
- [密钥管理最佳实践](https://docs.github.com/en/actions/security-guides/security-hardening-for-github-actions)

---

## 配置完成

完成上述配置后，你的 CI/CD 流程已经准备就绪！

下一步：参考 [快速发布指南](QUICK_RELEASE.md) 发布第一个版本。
