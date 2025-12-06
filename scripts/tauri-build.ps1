# Tauri Build Script with Environment Variables
# 用于加载 .env.local 并执行 tauri build

param(
    [switch]$Debug,
    [switch]$Help
)

if ($Help) {
    Write-Host "Tauri Build Script - 自动加载环境变量并构建"
    Write-Host ""
    Write-Host "用法:"
    Write-Host "  .\scripts\tauri-build.ps1          # 发布版本构建"
    Write-Host "  .\scripts\tauri-build.ps1 -Debug   # 调试版本构建"
    Write-Host "  .\scripts\tauri-build.ps1 -Help    # 显示帮助"
    Write-Host ""
    Write-Host "说明:"
    Write-Host "  - 自动从 .env.local 加载环境变量"
    Write-Host "  - 设置 TAURI_SIGNING_PRIVATE_KEY 用于签名"
    Write-Host "  - 设置 TAURI_SIGNING_PRIVATE_KEY_PASSWORD (如果存在)"
    exit 0
}

# 检查 .env.local 是否存在
$envFile = Join-Path $PSScriptRoot ".." ".env.local"
if (-not (Test-Path $envFile)) {
    Write-Host "错误: .env.local 文件不存在" -ForegroundColor Red
    Write-Host "请先创建 .env.local 文件并配置 TAURI_SIGNING_PRIVATE_KEY" -ForegroundColor Yellow
    exit 1
}

Write-Host "正在加载环境变量..." -ForegroundColor Cyan

# 读取并解析 .env.local
Get-Content $envFile | ForEach-Object {
    $line = $_.Trim()
    # 跳过注释和空行
    if ($line -and -not $line.StartsWith("#")) {
        $parts = $line -split "=", 2
        if ($parts.Length -eq 2) {
            $key = $parts[0].Trim()
            $value = $parts[1].Trim()
            
            # 设置环境变量
            Set-Item -Path "env:$key" -Value $value
            Write-Host "  ✓ 已设置: $key" -ForegroundColor Green
        }
    }
}

# 验证必需的环境变量
if (-not $env:TAURI_SIGNING_PRIVATE_KEY) {
    Write-Host "错误: TAURI_SIGNING_PRIVATE_KEY 未设置" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "开始构建..." -ForegroundColor Cyan
Write-Host ""

# 执行构建
try {
    if ($Debug) {
        pnpm tauri build --debug
    } else {
        pnpm tauri build
    }
    
    if ($LASTEXITCODE -eq 0) {
        Write-Host ""
        Write-Host "✓ 构建成功!" -ForegroundColor Green
        Write-Host ""
        Write-Host "输出文件位置:" -ForegroundColor Cyan
        Write-Host "  src-tauri\target\release\bundle\nsis\*.exe" -ForegroundColor Yellow
    } else {
        Write-Host ""
        Write-Host "✗ 构建失败" -ForegroundColor Red
        exit $LASTEXITCODE
    }
} catch {
    Write-Host "构建过程中出错: $_" -ForegroundColor Red
    exit 1
}
