// Live Subtitles - 实时字幕应用
// 基于 Tauri v2 + sherpa-rs

#[cfg(not(target_os = "windows"))]
mod audio;
#[cfg(target_os = "windows")]
mod audio_wasapi;
mod config;
mod online_asr;

#[cfg(not(target_os = "windows"))]
use audio::AudioCapture;
#[cfg(target_os = "windows")]
use audio_wasapi::AudioCapture;
use config::AppConfig;
use config::ScannedModelFiles;
use cpal::traits::{DeviceTrait, HostTrait};
use online_asr::{OnlineRecognizer, OnlineRecognizerConfig};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::thread;
use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder,
};

/// 应用状态
pub struct AppState {
    /// 配置
    config: Mutex<AppConfig>,
    /// 音频捕获 (运行时创建)
    audio_capture: Mutex<Option<AudioCapture>>,
    /// 是否正在识别
    is_running: Mutex<bool>,
    /// 模型目录
    models_dir: PathBuf,
}

impl AppState {
    fn new(models_dir: PathBuf) -> Self {
        Self {
            config: Mutex::new(AppConfig::default()),
            audio_capture: Mutex::new(None),
            is_running: Mutex::new(false),
            models_dir,
        }
    }
}

/// 音频设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioDeviceInfo {
    /// 设备 ID (唯一标识)
    pub id: String,
    /// 设备名称
    pub name: String,
    /// 设备类型: "input" 或 "output"
    pub device_type: String,
    /// 是否是默认设备
    pub is_default: bool,
}

/// 发送给前端的字幕事件
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleEvent {
    /// 识别的文本
    pub text: String,
    /// 是否是句子结束 (endpoint)
    pub is_final: bool,
    /// 时间戳 (毫秒)
    pub timestamp: u64,
}

impl SubtitleEvent {
    fn new(text: String, is_final: bool) -> Self {
        Self {
            text,
            is_final,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
        }
    }
}

/// 枚举所有可用的音频设备
#[tauri::command]
async fn enumerate_audio_devices() -> Result<Vec<AudioDeviceInfo>, String> {
    let host = cpal::default_host();
    let mut devices = Vec::new();

    // 获取默认输入和输出设备的名称
    let default_input_name = host.default_input_device().and_then(|d| d.name().ok());

    let default_output_name = host.default_output_device().and_then(|d| d.name().ok());

    // 枚举所有设备
    let all_devices = host
        .devices()
        .map_err(|e| format!("Failed to enumerate devices: {}", e))?;

    for (index, device) in all_devices.enumerate() {
        let device_name = device
            .name()
            .unwrap_or_else(|_| "Unknown Device".to_string());

        // 使用索引+名称作为设备 ID
        let device_id = format!("{}:{}", index, device_name);

        // 检查设备支持的类型
        let supports_input = device.supports_input();
        let supports_output = device.supports_output();

        // 添加为输入设备（麦克风）
        if supports_input {
            let is_default = default_input_name.as_ref() == Some(&device_name);
            devices.push(AudioDeviceInfo {
                id: format!("input:{}", device_id),
                name: format!("{} (麦克风)", device_name),
                device_type: "input".to_string(),
                is_default,
            });
        }

        // 添加为输出设备（用于 loopback，捕获系统音频）
        if supports_output {
            let is_default = default_output_name.as_ref() == Some(&device_name);
            devices.push(AudioDeviceInfo {
                id: format!("output:{}", device_id),
                name: format!("{} (系统音频)", device_name),
                device_type: "output".to_string(),
                is_default,
            });
        }
    }

    Ok(devices)
}

/// 获取可用的模型列表
#[tauri::command]
async fn get_available_models(_state: State<'_, Arc<AppState>>) -> Result<Vec<String>, String> {
    // TODO: 扫描 models 目录，返回可用的模型列表
    // 目前只返回默认的 zipformer-small-bilingual
    Ok(vec!["zipformer-small-bilingual".to_string()])
}

/// 获取模型目录路径
#[tauri::command]
async fn get_models_dir(state: State<'_, Arc<AppState>>) -> Result<String, String> {
    Ok(state.models_dir.to_string_lossy().to_string())
}

/// 获取当前配置
#[tauri::command]
async fn get_config(state: State<'_, Arc<AppState>>) -> Result<AppConfig, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

/// 更新配置
#[tauri::command]
async fn update_config(state: State<'_, Arc<AppState>>, config: AppConfig) -> Result<(), String> {
    let mut current_config = state.config.lock().map_err(|e| e.to_string())?;
    *current_config = config;
    Ok(())
}

/// 扫描模型文件夹，自动识别模型文件
#[tauri::command]
async fn scan_model_dir(dir_path: String) -> Result<ScannedModelFiles, String> {
    let path = PathBuf::from(&dir_path);
    ScannedModelFiles::scan_directory(&path).ok_or_else(|| format!("无法扫描目录: {}", dir_path))
}

/// 扫描模型根目录，返回所有可用的模型列表
#[tauri::command]
async fn scan_models_root_dir(root_dir: String) -> Result<Vec<ScannedModelFiles>, String> {
    let root_path = PathBuf::from(&root_dir);

    if !root_path.is_dir() {
        return Err(format!("目录不存在: {}", root_dir));
    }

    let mut models = Vec::new();

    // 遍历根目录下的所有子目录
    if let Ok(entries) = std::fs::read_dir(&root_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // 扫描每个子目录
                if let Some(model) = ScannedModelFiles::scan_directory(&path) {
                    models.push(model);
                }
            }
        }
    }

    // 按模型名称排序
    models.sort_by(|a, b| a.model_name.cmp(&b.model_name));

    Ok(models)
}

/// 获取识别状态
#[tauri::command]
async fn is_recognition_running(state: State<'_, Arc<AppState>>) -> Result<bool, String> {
    let is_running = state.is_running.lock().map_err(|e| e.to_string())?;
    Ok(*is_running)
}

/// 开始识别
#[tauri::command]
async fn start_recognition(
    app_handle: tauri::AppHandle,
    state: State<'_, Arc<AppState>>,
) -> Result<(), String> {
    // 检查是否已经在运行
    {
        let is_running = state.is_running.lock().map_err(|e| e.to_string())?;
        if *is_running {
            return Err("Recognition is already running".to_string());
        }
    }

    // 获取配置
    let config = {
        let config = state.config.lock().map_err(|e| e.to_string())?;
        config.clone()
    };

    // 获取当前模型配置
    let asr_config = config
        .current_model()
        .ok_or_else(|| "No ASR model configured".to_string())?
        .clone();

    // 打印当前使用的模型信息
    println!("========================================");
    println!("Starting recognition with model:");
    println!("  Model ID: {}", asr_config.id);
    println!("  Model Name: {}", asr_config.name);
    println!("  Model Dir: {}", asr_config.model_dir);
    match &asr_config.model_type {
        crate::config::AsrModelType::Transducer {
            encoder,
            decoder,
            joiner,
        } => {
            println!("  Type: Transducer");
            println!("  Encoder: {}", encoder);
            println!("  Decoder: {}", decoder);
            println!("  Joiner: {}", joiner);
        }
        crate::config::AsrModelType::Paraformer { model } => {
            println!("  Type: Paraformer");
            println!("  Model: {}", model);
        }
        crate::config::AsrModelType::Whisper { encoder, decoder } => {
            println!("  Type: Whisper");
            println!("  Encoder: {}", encoder);
            println!("  Decoder: {}", decoder);
        }
        crate::config::AsrModelType::SenseVoice { model } => {
            println!("  Type: SenseVoice");
            println!("  Model: {}", model);
        }
    }
    println!("  Tokens: {}", asr_config.tokens);
    println!("Audio Source:");
    println!("  Type: {:?}", config.audio_source_type);
    if config.audio_device_id.is_empty() {
        println!("  Device: Default");
    } else {
        println!("  Device ID: {}", config.audio_device_id);
    }
    println!("========================================");

    // 创建音频捕获（根据配置选择捕获模式）
    #[cfg(target_os = "windows")]
    use audio_wasapi::CaptureMode;

    #[cfg(target_os = "windows")]
    let (capture_mode, device_id) = match config.audio_source_type {
        config::AudioSourceType::SystemAudio => {
            // 系统音频始终使用默认输出设备
            (CaptureMode::SystemAudio, None)
        }
        config::AudioSourceType::Microphone => {
            // 麦克风使用用户选择的设备ID
            let device_id = if config.audio_device_id.is_empty() {
                None
            } else {
                Some(config.audio_device_id.clone())
            };
            (CaptureMode::Microphone, device_id)
        }
    };

    #[cfg(target_os = "windows")]
    let mut audio_capture =
        AudioCapture::new_with_device(asr_config.sample_rate, capture_mode, device_id);

    #[cfg(not(target_os = "windows"))]
    let mut audio_capture = AudioCapture::new(asr_config.sample_rate);

    let audio_rx = audio_capture
        .start()
        .map_err(|e| format!("Failed to start audio capture: {}", e))?;

    // 保存音频捕获实例
    {
        let mut audio = state.audio_capture.lock().map_err(|e| e.to_string())?;
        *audio = Some(audio_capture);
    }

    // 标记为运行中
    {
        let mut is_running = state.is_running.lock().map_err(|e| e.to_string())?;
        *is_running = true;
    }

    // 在后台线程中运行识别
    let models_dir = state.models_dir.clone();
    let state_clone = Arc::clone(&state.inner());

    thread::spawn(move || {
        // 构建 OnlineRecognizer 配置
        let (encoder, decoder, joiner) = match &asr_config.model_type {
            config::AsrModelType::Transducer {
                encoder,
                decoder,
                joiner,
            } => (
                models_dir.join(encoder).to_string_lossy().to_string(),
                models_dir.join(decoder).to_string_lossy().to_string(),
                models_dir.join(joiner).to_string_lossy().to_string(),
            ),
            _ => {
                eprintln!("OnlineRecognizer only supports Transducer models");
                let _ =
                    app_handle.emit("recognition_error", "Only Transducer models are supported");
                return;
            }
        };

        let online_config = OnlineRecognizerConfig {
            encoder,
            decoder,
            joiner,
            tokens: models_dir
                .join(&asr_config.tokens)
                .to_string_lossy()
                .to_string(),
            sample_rate: asr_config.sample_rate as i32,
            feature_dim: 80,
            num_threads: asr_config.num_threads,
            enable_endpoint: true,
            rule1_min_trailing_silence: 2.4,  // 句子结束静音
            rule2_min_trailing_silence: 1.2,  // 中间停顿静音
            rule3_min_utterance_length: 20.0, // 最小语句长度
            decoding_method: "greedy_search".to_string(),
            debug: false, // 关闭 debug 模式减少日志输出
        };

        // 通知前端开始加载模型
        let _ = app_handle.emit("model_loading", serde_json::json!({"loading": true}));

        // 创建 OnlineRecognizer
        match OnlineRecognizer::new(online_config) {
            Ok(recognizer) => {
                // 模型加载完成
                let _ = app_handle.emit("model_loading", serde_json::json!({"loading": false}));
                let mut last_text = String::new();

                // 循环处理音频
                while let Ok(samples) = audio_rx.recv() {
                    // 检查是否仍在运行
                    if let Ok(is_running) = state_clone.is_running.lock() {
                        if !*is_running {
                            break;
                        }
                    }

                    // 处理音频
                    let (text, is_endpoint) = recognizer.process(&samples);

                    // 如果有新文本，发送更新
                    if !text.is_empty() && text != last_text {
                        // 中间结果，不是最终的
                        let event = SubtitleEvent::new(text.clone(), false);
                        let _ = app_handle.emit("subtitle", &event);
                        last_text = text.clone();
                    }

                    // 如果到达 endpoint，发送最终结果并重置流
                    if is_endpoint && !last_text.is_empty() {
                        // 发送最终结果
                        let event = SubtitleEvent::new(last_text.clone(), true);
                        let _ = app_handle.emit("subtitle", &event);

                        recognizer.reset();
                        last_text.clear();
                    } else if is_endpoint {
                        // 没有文本但检测到 endpoint，只重置
                        recognizer.reset();
                    }
                }
            }
            Err(e) => {
                eprintln!("Failed to create OnlineRecognizer: {}", e);
                // 模型加载失败，取消加载状态
                let _ = app_handle.emit("model_loading", serde_json::json!({"loading": false}));
                let _ = app_handle.emit("recognition_error", &e);
            }
        }

        // 清理状态
        if let Ok(mut is_running) = state_clone.is_running.lock() {
            *is_running = false;
        }
    });

    Ok(())
}

/// 停止识别
#[tauri::command]
async fn stop_recognition(state: State<'_, Arc<AppState>>) -> Result<(), String> {
    // 标记为停止
    {
        let mut is_running = state.is_running.lock().map_err(|e| e.to_string())?;
        *is_running = false;
    }

    // 停止音频捕获
    {
        let mut audio = state.audio_capture.lock().map_err(|e| e.to_string())?;
        if let Some(mut capture) = audio.take() {
            capture.stop();
        }
    }

    Ok(())
}

/// 打开设置窗口
#[tauri::command]
async fn open_settings(app: tauri::AppHandle) -> Result<(), String> {
    // 检查设置窗口是否已存在
    if let Some(window) = app.get_webview_window("settings") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
        return Ok(());
    }

    // 创建新的设置窗口，使用同一个 index.html 但路由到 /settings
    WebviewWindowBuilder::new(&app, "settings", WebviewUrl::App("/settings".into()))
        .title("设置")
        .inner_size(600.0, 500.0)
        .resizable(true)
        .center()
        .build()
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 获取样式文件路径
#[tauri::command]
async fn get_style_path(app: tauri::AppHandle) -> Result<String, String> {
    let styles_dir = if cfg!(debug_assertions) {
        // 开发环境：使用 src-tauri/resources/styles
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("styles")
    } else {
        // 生产环境：使用资源目录
        app.path()
            .resource_dir()
            .map_err(|e| e.to_string())?
            .join("styles")
    };

    let style_path = styles_dir.join("subtitle.css");
    Ok(style_path.to_string_lossy().to_string())
}

/// 打开样式编辑器（打开 devtools 并用默认程序打开 CSS 文件）
#[tauri::command]
async fn open_style_editor(app: tauri::AppHandle) -> Result<String, String> {
    // 获取样式文件路径
    let styles_dir = if cfg!(debug_assertions) {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("resources")
            .join("styles")
    } else {
        app.path()
            .resource_dir()
            .map_err(|e| e.to_string())?
            .join("styles")
    };

    let style_path = styles_dir.join("subtitle.css");

    // 打开主窗口的 devtools（需要在 Cargo.toml 中启用 devtools feature）
    if let Some(window) = app.get_webview_window("main") {
        window.open_devtools();
    }

    // 用默认程序打开 CSS 文件
    if style_path.exists() {
        #[cfg(target_os = "windows")]
        {
            std::process::Command::new("cmd")
                .args(["/C", "start", "", &style_path.to_string_lossy()])
                .spawn()
                .map_err(|e| format!("无法打开文件: {}", e))?;
        }
        #[cfg(target_os = "macos")]
        {
            std::process::Command::new("open")
                .arg(&style_path)
                .spawn()
                .map_err(|e| format!("无法打开文件: {}", e))?;
        }
        #[cfg(target_os = "linux")]
        {
            std::process::Command::new("xdg-open")
                .arg(&style_path)
                .spawn()
                .map_err(|e| format!("无法打开文件: {}", e))?;
        }
    }

    Ok(style_path.to_string_lossy().to_string())
}

/// 显示主窗口
#[tauri::command]
async fn show_main_window(app: tauri::AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window("main") {
        window.show().map_err(|e| e.to_string())?;
        window.set_focus().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            // 获取模型目录：开发时使用 src-tauri/resources/models，生产环境使用资源目录
            let models_dir = if cfg!(debug_assertions) {
                // 开发环境：使用 src-tauri/resources/models
                PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .join("resources")
                    .join("models")
            } else {
                // 生产环境：使用资源目录
                app.path()
                    .resource_dir()
                    .expect("Failed to get resource dir")
                    .join("models")
            };

            // 创建应用状态
            let state = Arc::new(AppState::new(models_dir));
            app.manage(state);

            // 创建托盘菜单
            let show_item = MenuItem::with_id(app, "show", "显示字幕", true, None::<&str>)?;
            let settings_item = MenuItem::with_id(app, "settings", "设置", true, None::<&str>)?;
            let restart_item = MenuItem::with_id(app, "restart", "重启", true, None::<&str>)?;
            let quit_item = MenuItem::with_id(app, "quit", "退出", true, None::<&str>)?;
            let menu = Menu::with_items(
                app,
                &[&show_item, &settings_item, &restart_item, &quit_item],
            )?;

            // 创建托盘图标
            let _tray = TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .show_menu_on_left_click(false)
                .tooltip("Live Subtitles - 实时字幕")
                .on_menu_event(|app, event| match event.id.as_ref() {
                    "show" => {
                        if let Some(window) = app.get_webview_window("main") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                    "settings" => {
                        // 检查设置窗口是否已存在
                        if let Some(window) = app.get_webview_window("settings") {
                            let _ = window.show();
                            let _ = window.set_focus();
                        } else {
                            // 创建新的设置窗口（无边框）
                            let _ = WebviewWindowBuilder::new(
                                app,
                                "settings",
                                WebviewUrl::App("/settings".into()),
                            )
                            .title("设置")
                            .inner_size(650.0, 550.0)
                            .resizable(true)
                            .decorations(false)
                            .center()
                            .build();
                        }
                    }
                    "restart" => {
                        app.restart();
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        let app = tray.app_handle();
                        // 左键点击切换设置窗口显示/隐藏
                        if let Some(window) = app.get_webview_window("settings") {
                            if window.is_visible().unwrap_or(false) {
                                let _ = window.hide();
                            } else {
                                let _ = window.show();
                                let _ = window.set_focus();
                            }
                        } else {
                            // 创建新的设置窗口（无边框）
                            let _ = WebviewWindowBuilder::new(
                                app,
                                "settings",
                                WebviewUrl::App("/settings".into()),
                            )
                            .title("设置")
                            .inner_size(650.0, 550.0)
                            .resizable(true)
                            .decorations(false)
                            .center()
                            .build();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            enumerate_audio_devices,
            get_available_models,
            get_models_dir,
            get_config,
            update_config,
            scan_model_dir,
            scan_models_root_dir,
            is_recognition_running,
            start_recognition,
            stop_recognition,
            open_settings,
            show_main_window,
            get_style_path,
            open_style_editor,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
