# 验证更新签名密钥配置

Write-Host "正在验证更新签名密钥配置..." -ForegroundColor Cyan
Write-Host ""

# 检查密钥文件
$privateKeyPath = Join-Path $env:USERPROFILE ".tauri\my-tools.key"
$publicKeyPath = Join-Path $env:USERPROFILE ".tauri\my-tools.key.pub"

Write-Host "1. 检查密钥文件是否存在..." -ForegroundColor Yellow
if (Test-Path $privateKeyPath) {
    Write-Host "   ✓ 私钥文件存在: $privateKeyPath" -ForegroundColor Green
    $privateKeyContent = Get-Content $privateKeyPath -Raw
    Write-Host "   私钥长度: $($privateKeyContent.Length) 字符" -ForegroundColor Gray
} else {
    Write-Host "   ✗ 私钥文件不存在!" -ForegroundColor Red
    exit 1
}

if (Test-Path $publicKeyPath) {
    Write-Host "   ✓ 公钥文件存在: $publicKeyPath" -ForegroundColor Green
    $publicKeyContent = Get-Content $publicKeyPath -Raw
    Write-Host "   公钥长度: $($publicKeyContent.Length) 字符" -ForegroundColor Gray
} else {
    Write-Host "   ✗ 公钥文件不存在!" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "2. 显示密钥内容用于 GitHub Secrets 配置..." -ForegroundColor Yellow
Write-Host ""
Write-Host "========== 复制以下内容到 TAURI_SIGNING_PRIVATE_KEY ==========" -ForegroundColor Cyan
Write-Host $privateKeyContent.Trim()
Write-Host "=================================================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "提示: 私钥应该是 348 个字符的单行文本" -ForegroundColor Gray
Write-Host "     确保复制时没有多余的空格或换行符!" -ForegroundColor Gray
Write-Host ""

Write-Host "3. 检查 tauri.conf.json 中的公钥配置..." -ForegroundColor Yellow
$configPath = Join-Path $PSScriptRoot "src-tauri\tauri.conf.json"
if (Test-Path $configPath) {
    $config = Get-Content $configPath -Raw | ConvertFrom-Json
    $configPubkey = $config.plugins.updater.pubkey
    $filePubkey = $publicKeyContent.Trim()
    
    if ($configPubkey -eq $filePubkey) {
        Write-Host "   ✓ 公钥配置正确" -ForegroundColor Green
    } else {
        Write-Host "   ✗ 公钥配置不匹配!" -ForegroundColor Red
        Write-Host "   配置文件中的公钥长度: $($configPubkey.Length)" -ForegroundColor Gray
        Write-Host "   密钥文件中的公钥长度: $($filePubkey.Length)" -ForegroundColor Gray
    }
} else {
    Write-Host "   ✗ 找不到 tauri.conf.json!" -ForegroundColor Red
}

Write-Host ""
Write-Host "4. 检查 GitHub Actions 工作流配置..." -ForegroundColor Yellow
$workflowPath = Join-Path $PSScriptRoot ".github\workflows\build.yml"
if (Test-Path $workflowPath) {
    $workflowContent = Get-Content $workflowPath -Raw
    
    if ($workflowContent -match 'TAURI_SIGNING_PRIVATE_KEY:') {
        Write-Host "   ✓ TAURI_SIGNING_PRIVATE_KEY 已配置" -ForegroundColor Green
    } else {
        Write-Host "   ✗ TAURI_SIGNING_PRIVATE_KEY 未配置!" -ForegroundColor Red
    }
    
    if ($workflowContent -match 'TAURI_SIGNING_PRIVATE_KEY_PASSWORD:') {
        Write-Host "   ✓ TAURI_SIGNING_PRIVATE_KEY_PASSWORD 已配置" -ForegroundColor Green
    } else {
        Write-Host "   ✗ TAURI_SIGNING_PRIVATE_KEY_PASSWORD 未配置!" -ForegroundColor Red
    }
} else {
    Write-Host "   ✗ 找不到 build.yml!" -ForegroundColor Red
}

Write-Host ""
Write-Host "5. 检查 bundle.createUpdaterArtifacts 配置..." -ForegroundColor Yellow
if (Test-Path $configPath) {
    $config = Get-Content $configPath -Raw | ConvertFrom-Json
    if ($config.bundle.createUpdaterArtifacts -eq $true) {
        Write-Host "   ✓ createUpdaterArtifacts 已启用" -ForegroundColor Green
    } else {
        Write-Host "   ✗ createUpdaterArtifacts 未启用!" -ForegroundColor Red
    }
}

Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "验证完成!" -ForegroundColor Green
Write-Host ""
Write-Host "下一步操作:" -ForegroundColor Yellow
Write-Host "1. 复制上面显示的私钥到 GitHub Secrets" -ForegroundColor White
Write-Host "2. 配置 TAURI_SIGNING_PRIVATE_KEY_PASSWORD (你设置的密码)" -ForegroundColor White
Write-Host "3. 如果忘记密码,运行: pnpm tauri signer generate -w `$env:USERPROFILE\.tauri\my-tools.key --force" -ForegroundColor White
Write-Host "========================================" -ForegroundColor Cyan
