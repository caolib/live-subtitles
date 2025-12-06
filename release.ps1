# 发布脚本 - 自动化版本发布流程
param(
    [Parameter()]
    [string]$Version
)

#region 参数验证
# 交互式菜单选择函数
function Show-InteractiveMenu {
    param(
        [string[]]$Options,
        [string]$Title = "请选择选项"
    )
    
    $selectedIndex = 0
    $lastSelectedIndex = -1
    
    # 初始显示
    Clear-Host
    Write-Host $Title -ForegroundColor Cyan
    Write-Host ""
    
    # 记录菜单开始的行位置
    $menuStartRow = [Console]::CursorTop
    
    for ($i = 0; $i -lt $Options.Length; $i++) {
        if ($i -eq $selectedIndex) {
            Write-Host "→ $($Options[$i])" -ForegroundColor Green
        } else {
            Write-Host "  $($Options[$i])" -ForegroundColor White
        }
    }
    
    Write-Host ""
    Write-Host "使用 ↑↓ 键选择，Enter 确认，Esc 取消" -ForegroundColor Gray
    
    # 交互选择
    while ($true) {
        $key = $Host.UI.RawUI.ReadKey("NoEcho,IncludeKeyDown")
        
        if ($key.VirtualKeyCode -eq 27) {  # Esc
            return -1
        }
        
        if ($key.VirtualKeyCode -eq 13) {  # Enter
            return $selectedIndex
        }
        
        if ($key.VirtualKeyCode -eq 38) {  # Up Arrow
            $selectedIndex = ($selectedIndex - 1 + $Options.Length) % $Options.Length
        }
        
        if ($key.VirtualKeyCode -eq 40) {  # Down Arrow
            $selectedIndex = ($selectedIndex + 1) % $Options.Length
        }
        
        # 只在选择项变化时重绘
        if ($selectedIndex -ne $lastSelectedIndex) {
            [Console]::SetCursorPosition(0, $menuStartRow)
            
            # 重绘菜单选项
            for ($i = 0; $i -lt $Options.Length; $i++) {
                if ($i -eq $selectedIndex) {
                    Write-Host "→ $($Options[$i])" -ForegroundColor Green
                } else {
                    Write-Host "  $($Options[$i])" -ForegroundColor White
                }
            }
            
            $lastSelectedIndex = $selectedIndex
        }
    }
}

# 显示最新的tag并生成预设选项
try {
    $latestTag = git describe --tags --abbrev=0 2>$null
    if ($LASTEXITCODE -eq 0 -and $latestTag) {
        Write-Host "当前最新的标签: $latestTag" -ForegroundColor Green
        
        # 解析版本号 (去除 v 前缀)
        $versionNum = $latestTag -replace '^v', ''
        if ($versionNum -match '^(\d+)\.(\d+)\.(\d+)') {
            $major = [int]$matches[1]
            $minor = [int]$matches[2]
            $patch = [int]$matches[3]
            
            # 生成预设版本选项
            $patchVersion = "$major.$minor.$($patch + 1)"
            $minorVersion = "$major.$($minor + 1).0"
            $majorVersion = "$($major + 1).0.0"
            
            $options = @(
                "$patchVersion (补丁版本 - bug修复)",
                "$minorVersion (次要版本 - 新功能)",
                "$majorVersion (主要版本 - 重大更新)",
                "手动输入版本号"
            )
            
            $choice = Show-InteractiveMenu -Options $options -Title "选择版本类型 (当前: $latestTag)"
            
            if ($choice -eq -1) {
                Write-Host "已取消操作" -ForegroundColor Yellow
                exit 0
            }
            
            switch ($choice) {
                0 { $Version = $patchVersion }
                1 { $Version = $minorVersion }
                2 { $Version = $majorVersion }
                3 { 
                    $Version = Read-Host "请输入版本号 (例如: 1.2.3)"
                }
            }
        }
    }
} catch {
    # 忽略错误，继续执行
}

# 如果还没有版本号，提示输入
if (-not $Version) {
    $Version = Read-Host "请输入版本号 (例如: 0.1.0)"
}

# 验证版本号格式
if ($Version -notmatch '^\d+\.\d+\.\d+$') {
    Write-Host "错误: 版本号格式不正确，应该是 x.y.z 格式 (例如: 0.1.0)" -ForegroundColor Red
    exit 1
}

Write-Host "即将发布版本: $Version" -ForegroundColor Green
#endregion

#region 更新版本号
Write-Host ""
Write-Host "正在更新版本号..." -ForegroundColor Cyan

# 更新 tauri.conf.json
$configPath = "src-tauri\tauri.conf.json"
$config = Get-Content $configPath -Raw | ConvertFrom-Json
$config.version = $Version
$config | ConvertTo-Json -Depth 100 | Set-Content $configPath -Encoding UTF8

# 更新 Cargo.toml
$cargoPath = "src-tauri\Cargo.toml"
$cargoContent = Get-Content $cargoPath -Raw
$cargoContent = $cargoContent -replace 'version\s*=\s*"[^"]*"', "version = `"$Version`""
$cargoContent | Set-Content $cargoPath -Encoding UTF8 -NoNewline

# 更新 package.json
$packagePath = "package.json"
$package = Get-Content $packagePath -Raw | ConvertFrom-Json
$package.version = $Version
$package | ConvertTo-Json -Depth 100 | Set-Content $packagePath -Encoding UTF8

Write-Host "✓ 版本号已更新为 $Version" -ForegroundColor Green
#endregion

#region 生成发布说明
Write-Host ""
Write-Host "正在生成发布说明..." -ForegroundColor Cyan

$releaseNotes = @"
## 版本 $Version

### 更新内容

- 待补充...

### 修复

- 待补充...

---
📋 [查看完整更新日志](https://github.com/caolib/live-subtitles/compare/v$Version...main)
"@

# 创建 docs 目录（如果不存在）
$docsPath = "docs"
if (-not (Test-Path $docsPath)) {
    New-Item -ItemType Directory -Path $docsPath | Out-Null
}

$releaseNotesPath = "$docsPath\RELEASE.md"
$releaseNotes | Set-Content $releaseNotesPath -Encoding UTF8

Write-Host "✓ 发布说明已生成: $releaseNotesPath" -ForegroundColor Green
Write-Host ""
Write-Host "请编辑 $releaseNotesPath 文件，添加具体的更新内容" -ForegroundColor Yellow
Write-Host "编辑完成后按 Enter 继续..." -ForegroundColor Yellow
Read-Host
#endregion

#region 提交并推送
Write-Host ""
Write-Host "正在提交更改..." -ForegroundColor Cyan

git add .
git commit -m "chore: 发布版本 $Version"

if ($LASTEXITCODE -ne 0) {
    Write-Host "错误: 提交失败" -ForegroundColor Red
    exit 1
}

Write-Host "✓ 更改已提交" -ForegroundColor Green

Write-Host ""
Write-Host "正在推送到远程仓库..." -ForegroundColor Cyan

git push

if ($LASTEXITCODE -ne 0) {
    Write-Host "错误: 推送失败" -ForegroundColor Red
    exit 1
}

Write-Host "✓ 代码已推送" -ForegroundColor Green
#endregion

#region 创建并推送标签
Write-Host ""
Write-Host "正在创建标签 $Version..." -ForegroundColor Cyan

git tag $Version

if ($LASTEXITCODE -ne 0) {
    Write-Host "错误: 创建标签失败" -ForegroundColor Red
    exit 1
}

Write-Host "✓ 标签已创建" -ForegroundColor Green

Write-Host ""
Write-Host "正在推送标签..." -ForegroundColor Cyan

git push origin $Version

if ($LASTEXITCODE -ne 0) {
    Write-Host "错误: 推送标签失败" -ForegroundColor Red
    exit 1
}

Write-Host "✓ 标签已推送" -ForegroundColor Green
#endregion

#region 完成
Write-Host ""
Write-Host "========================================" -ForegroundColor Cyan
Write-Host "🎉 版本 $Version 发布流程完成！" -ForegroundColor Green
Write-Host "========================================" -ForegroundColor Cyan
Write-Host ""
Write-Host "后续步骤:" -ForegroundColor Yellow
Write-Host "1. 访问 https://github.com/caolib/live-subtitles/actions 查看构建进度" -ForegroundColor White
Write-Host "2. 构建完成后，在 https://github.com/caolib/live-subtitles/releases 查看发布" -ForegroundColor White
Write-Host "3. 验证安装包和 latest.json 文件是否正确生成" -ForegroundColor White
Write-Host ""
#endregion
