; NSIS installer hooks for live-subtitles
; 保护用户的 models 目录不被覆盖

!macro NSIS_HOOK_PREINSTALL
  ; 在安装前备份 models 目录（如果存在）
  ${If} ${FileExists} "$INSTDIR\models\*.*"
    DetailPrint "Backing up existing models directory..."
    
    ; 创建临时备份目录
    CreateDirectory "$TEMP\live-subtitles-models-backup"
    
    ; 复制所有模型文件到临时目录
    CopyFiles /SILENT "$INSTDIR\models\*.*" "$TEMP\live-subtitles-models-backup"
    
    DetailPrint "Models backed up to temporary directory"
  ${EndIf}
!macroend

!macro NSIS_HOOK_POSTINSTALL
  ; 安装后恢复用户的模型文件
  ${If} ${FileExists} "$TEMP\live-subtitles-models-backup\*.*"
    DetailPrint "Restoring models directory..."
    
    ; 恢复模型文件（覆盖空的 .gitkeep）
    CopyFiles /SILENT "$TEMP\live-subtitles-models-backup\*.*" "$INSTDIR\models"
    
    ; 清理临时备份
    RMDir /r "$TEMP\live-subtitles-models-backup"
    
    DetailPrint "Models restored successfully"
  ${EndIf}
!macroend

!macro NSIS_HOOK_PREUNINSTALL
  ; 卸载前不删除模型文件，保留用户数据
  DetailPrint "Preserving user models..."
!macroend

!macro NSIS_HOOK_POSTUNINSTALL
  ; 卸载后保留 models 目录
  ; NSIS 默认会尝试删除空目录，但不会删除有内容的目录
  DetailPrint "User models have been preserved"
!macroend
