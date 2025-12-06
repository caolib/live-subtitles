# Tauri Build Script - 加载环境变量并构建
# 使用方法: .\scripts\build.ps1

$ErrorActionPreference = "Stop"

Write-Host "正在加载环境变量..." -ForegroundColor Cyan

# 检查 .env 文件
$envFile = Join-Path $PSScriptRoot ".." ".env"
if (-not (Test-Path $envFile)) {
    Write-Host "错误: .env 文件不存在" -ForegroundColor Red
    Write-Host "请先创建 .env 文件并配置签名密钥" -ForegroundColor Yellow
    exit 1
}

# 读取并设置环境变量
Get-Content $envFile | ForEach-Object {
    $line = $_.Trim()
    if ($line -and -not $line.StartsWith("#")) {
        if ($line -match "^([^=]+)=(.*)$") {
            $key = $matches[1].Trim()
            $value = $matches[2].Trim()
            [System.Environment]::SetEnvironmentVariable($key, $value, [System.EnvironmentVariableTarget]::Process)
            Write-Host "  ✓ $key" -ForegroundColor Green
        }
    }
}

# 验证
if (-not $env:TAURI_SIGNING_PRIVATE_KEY) {
    Write-Host "错误: TAURI_SIGNING_PRIVATE_KEY 未设置" -ForegroundColor Red
    exit 1
}

Write-Host ""
Write-Host "开始构建..." -ForegroundColor Cyan
Write-Host ""

# 执行构建
pnpm tauri build

if ($LASTEXITCODE -eq 0) {
    Write-Host ""
    Write-Host "✓ 构建成功!" -ForegroundColor Green
} else {
    exit $LASTEXITCODE
}
