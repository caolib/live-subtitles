//! WASAPI Loopback 音频捕获模块
//! 使用 Windows Audio Session API 捕获系统音频

use rubato::{FftFixedIn, Resampler};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread;
use windows::Win32::Media::Audio::*;
use windows::Win32::System::Com::*;

/// 音频捕获模式
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CaptureMode {
    /// 系统音频 (Loopback)
    SystemAudio,
    /// 麦克风输入
    Microphone,
}

/// 音频捕获器
pub struct AudioCapture {
    stop_flag: Arc<Mutex<bool>>,
    capture_thread: Option<thread::JoinHandle<()>>,
    target_sample_rate: u32,
    capture_mode: CaptureMode,
    device_id: Option<String>,
}

/// 音频数据接收器
pub type AudioReceiver = Receiver<Vec<f32>>;

impl AudioCapture {
    /// 创建音频捕获器
    ///
    /// # 参数
    /// * `target_sample_rate` - 目标采样率
    /// * `capture_mode` - 捕获模式（系统音频或麦克风）
    /// * `device_id` - 设备ID（None 表示使用默认设备）
    pub fn new(target_sample_rate: u32) -> Self {
        Self::new_with_device(target_sample_rate, CaptureMode::SystemAudio, None)
    }

    /// 创建指定设备的音频捕获器
    pub fn new_with_device(
        target_sample_rate: u32,
        capture_mode: CaptureMode,
        device_id: Option<String>,
    ) -> Self {
        Self {
            stop_flag: Arc::new(Mutex::new(false)),
            capture_thread: None,
            target_sample_rate,
            capture_mode,
            device_id,
        }
    }

    /// 开始捕获音频
    pub fn start(&mut self) -> Result<AudioReceiver, String> {
        let (tx, rx) = mpsc::channel();
        let stop_flag = self.stop_flag.clone();
        let target_sample_rate = self.target_sample_rate;
        let capture_mode = self.capture_mode;
        let device_id = self.device_id.clone();

        // 重置停止标志
        *stop_flag.lock().unwrap() = false;

        let handle = thread::spawn(move || {
            let result = match capture_mode {
                CaptureMode::SystemAudio => {
                    capture_loopback_audio(tx, stop_flag, target_sample_rate, device_id)
                }
                CaptureMode::Microphone => {
                    capture_microphone_audio(tx, stop_flag, target_sample_rate, device_id)
                }
            };

            if let Err(e) = result {
                eprintln!("Audio capture error: {}", e);
            }
        });

        self.capture_thread = Some(handle);
        Ok(rx)
    }

    /// 停止捕获
    pub fn stop(&mut self) {
        *self.stop_flag.lock().unwrap() = true;
        if let Some(handle) = self.capture_thread.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        self.stop();
    }
}

/// WASAPI Loopback 捕获实现（系统音频）
fn capture_loopback_audio(
    tx: Sender<Vec<f32>>,
    stop_flag: Arc<Mutex<bool>>,
    target_sample_rate: u32,
    _device_id: Option<String>, // TODO: 支持选择特定设备
) -> Result<(), String> {
    unsafe {
        // 初始化 COM
        CoInitializeEx(Some(std::ptr::null()), COINIT_MULTITHREADED)
            .ok()
            .map_err(|e| format!("Failed to initialize COM: {:?}", e))?;

        // 获取音频设备枚举器
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| format!("Failed to create device enumerator: {}", e))?;

        // 获取默认的音频渲染设备（用于 loopback）
        let device = enumerator
            .GetDefaultAudioEndpoint(eRender, eConsole)
            .map_err(|e| format!("Failed to get default audio endpoint: {}", e))?;

        // 激活音频客户端
        let audio_client: IAudioClient = device
            .Activate(CLSCTX_ALL, None)
            .map_err(|e| format!("Failed to activate audio client: {}", e))?;

        // 获取混合格式
        let mix_format_ptr = audio_client
            .GetMixFormat()
            .map_err(|e| format!("Failed to get mix format: {}", e))?;

        let mix_format = &*mix_format_ptr;
        let source_sample_rate = mix_format.nSamplesPerSec;
        let channels = mix_format.nChannels as usize;
        let bits_per_sample = mix_format.wBitsPerSample;

        // 初始化音频客户端为 loopback 模式
        let buffer_duration = 10_000_000i64; // 1 秒 (100纳秒单位)
        audio_client
            .Initialize(
                AUDCLNT_SHAREMODE_SHARED,
                AUDCLNT_STREAMFLAGS_LOOPBACK,
                buffer_duration,
                0,
                mix_format_ptr,
                None,
            )
            .map_err(|e| format!("Failed to initialize audio client: {}", e))?;

        // 获取捕获客户端
        let capture_client: IAudioCaptureClient = audio_client
            .GetService()
            .map_err(|e| format!("Failed to get capture client: {}", e))?;

        // 启动捕获
        audio_client
            .Start()
            .map_err(|e| format!("Failed to start audio client: {}", e))?;

        // 创建重采样器
        let resampler = if source_sample_rate != target_sample_rate {
            // 使用 FftFixedIn，它允许可变输入大小
            let resampler = FftFixedIn::<f32>::new(
                source_sample_rate as usize,
                target_sample_rate as usize,
                2048, // max input chunk size
                2,    // sub chunks
                1,    // mono channel
            )
            .map_err(|e| format!("Failed to create resampler: {}", e))?;

            Some(Mutex::new(resampler))
        } else {
            None
        };

        let mut audio_buffer: Vec<f32> = Vec::new();
        // 使用重采样器需要的实际大小
        let chunk_size = if source_sample_rate != target_sample_rate {
            // 从 48000 -> 16000 的比例是 3:1
            // 我们需要足够的样本让重采样器工作
            2048
        } else {
            1024
        };

        // 捕获循环
        while !*stop_flag.lock().unwrap() {
            // 等待数据
            thread::sleep(std::time::Duration::from_millis(10));

            loop {
                let mut buffer_ptr: *mut u8 = std::ptr::null_mut();
                let mut num_frames = 0u32;
                let mut flags = 0u32;

                let hr = capture_client.GetBuffer(
                    &mut buffer_ptr,
                    &mut num_frames,
                    &mut flags,
                    None,
                    None,
                );

                if hr.is_err() || num_frames == 0 {
                    break;
                }

                // 转换为 f32 样本
                let samples = if bits_per_sample == 32 {
                    // 32-bit float
                    let float_ptr = buffer_ptr as *const f32;
                    std::slice::from_raw_parts(float_ptr, (num_frames as usize) * channels).to_vec()
                } else if bits_per_sample == 16 {
                    // 16-bit int
                    let int_ptr = buffer_ptr as *const i16;
                    let int_samples =
                        std::slice::from_raw_parts(int_ptr, (num_frames as usize) * channels);
                    int_samples.iter().map(|&s| s as f32 / 32768.0).collect()
                } else {
                    vec![]
                };

                // 释放缓冲区
                let _ = capture_client.ReleaseBuffer(num_frames);

                if samples.is_empty() {
                    continue;
                }

                // 检查是否是静音
                let is_silent = (flags & (AUDCLNT_BUFFERFLAGS_SILENT.0 as u32)) != 0;

                // 转换为单声道
                let mono_samples: Vec<f32> = if is_silent {
                    vec![0.0; num_frames as usize]
                } else {
                    samples
                        .chunks(channels)
                        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
                        .collect()
                };

                audio_buffer.extend(mono_samples);

                // 当缓冲区足够大时处理
                while audio_buffer.len() >= chunk_size {
                    let chunk: Vec<f32> = audio_buffer.drain(..chunk_size).collect();

                    let output = if let Some(ref resampler) = resampler {
                        let mut resampler_guard = resampler.lock().unwrap();
                        // 获取需要的输入帧数
                        let frames_needed = resampler_guard.input_frames_next();
                        if chunk.len() < frames_needed {
                            // 不够帧，跳过
                            continue;
                        }
                        match resampler_guard.process(&[chunk[..frames_needed].to_vec()], None) {
                            Ok(resampled) => resampled.into_iter().next().unwrap_or_default(),
                            Err(e) => {
                                eprintln!("[WASAPI] Resampler error: {}", e);
                                continue;
                            }
                        }
                    } else {
                        chunk
                    };

                    if tx.send(output).is_err() {
                        // 接收端已关闭
                        break;
                    }
                }
            }
        }

        // 停止捕获
        let _ = audio_client.Stop();
        CoUninitialize();

        Ok(())
    }
}

/// WASAPI 麦克风捕获实现
fn capture_microphone_audio(
    tx: Sender<Vec<f32>>,
    stop_flag: Arc<Mutex<bool>>,
    target_sample_rate: u32,
    _device_id: Option<String>, // TODO: 支持选择特定设备
) -> Result<(), String> {
    unsafe {
        println!("[Microphone] Starting microphone capture...");

        // 初始化 COM
        CoInitializeEx(Some(std::ptr::null()), COINIT_MULTITHREADED)
            .ok()
            .map_err(|e| format!("Failed to initialize COM: {:?}", e))?;

        // 获取音频设备枚举器
        let enumerator: IMMDeviceEnumerator =
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)
                .map_err(|e| format!("Failed to create device enumerator: {}", e))?;

        // 枚举所有活动的捕获设备（用于调试）
        // if let Ok(collection) = enumerator.EnumAudioEndpoints(eCapture, DEVICE_STATE_ACTIVE) {
        //     if let Ok(count) = collection.GetCount() {
        //         println!("[Microphone] Found {} active capture devices", count);
        //     }
        // }

        // 暂时使用默认设备（TODO: 后续支持设备选择）
        let device = enumerator
            .GetDefaultAudioEndpoint(eCapture, eConsole)
            .or_else(|_| {
                println!("[Microphone] Failed to get console device, trying communications");
                enumerator.GetDefaultAudioEndpoint(eCapture, eCommunications)
            })
            .map_err(|e| format!("Failed to get default capture endpoint: {}", e))?;

        // 激活音频客户端
        let audio_client: IAudioClient = device
            .Activate(CLSCTX_ALL, None)
            .map_err(|e| format!("Failed to activate audio client: {}", e))?;

        let mix_format_ptr = audio_client
            .GetMixFormat()
            .map_err(|e| format!("Failed to get mix format: {}", e))?;

        let mix_format = &*mix_format_ptr;
        let source_sample_rate = mix_format.nSamplesPerSec;
        let channels = mix_format.nChannels as usize;
        let bits_per_sample = mix_format.wBitsPerSample;

        println!(
            "[Microphone] Format: {} Hz, {} ch, {} bit",
            source_sample_rate, channels, bits_per_sample
        );

        // 尝试不同的初始化策略
        let buffer_duration = 10_000_000i64; // 1 second

        // 先尝试使用 NOPERSIST 标志（防止音频会话持久化）
        let init_result = audio_client.Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            AUDCLNT_STREAMFLAGS_NOPERSIST,
            buffer_duration,
            0,
            mix_format_ptr,
            None,
        );

        if init_result.is_err() {
            // 如果失败，尝试不使用任何标志
            audio_client
                .Initialize(
                    AUDCLNT_SHAREMODE_SHARED,
                    0,
                    buffer_duration,
                    0,
                    mix_format_ptr,
                    None,
                )
                .map_err(|e| format!("Failed to initialize audio client: {}", e))?;
        }

        // 获取捕获客户端
        let capture_client: IAudioCaptureClient = audio_client
            .GetService()
            .map_err(|e| format!("Failed to get capture client: {}", e))?;

        // 启动捕获
        audio_client
            .Start()
            .map_err(|e| format!("Failed to start audio client: {}", e))?;

        // 创建重采样器
        let resampler = if source_sample_rate != target_sample_rate {
            let resampler = FftFixedIn::<f32>::new(
                source_sample_rate as usize,
                target_sample_rate as usize,
                2048,
                2,
                1,
            )
            .map_err(|e| format!("Failed to create resampler: {}", e))?;

            Some(Mutex::new(resampler))
        } else {
            None
        };

        let mut audio_buffer: Vec<f32> = Vec::new();
        let chunk_size = if source_sample_rate != target_sample_rate {
            2048
        } else {
            1024
        };

        // 捕获循环
        while !*stop_flag.lock().unwrap() {
            thread::sleep(std::time::Duration::from_millis(10));

            loop {
                let mut buffer_ptr: *mut u8 = std::ptr::null_mut();
                let mut num_frames = 0u32;
                let mut flags = 0u32;

                let hr = capture_client.GetBuffer(
                    &mut buffer_ptr,
                    &mut num_frames,
                    &mut flags,
                    None,
                    None,
                );

                if hr.is_err() || num_frames == 0 {
                    break;
                }

                // 转换为 f32 样本
                let samples = if bits_per_sample == 32 {
                    let float_ptr = buffer_ptr as *const f32;
                    let raw_samples =
                        std::slice::from_raw_parts(float_ptr, (num_frames as usize) * channels);
                    raw_samples.to_vec()
                } else if bits_per_sample == 16 {
                    let int_ptr = buffer_ptr as *const i16;
                    let int_samples =
                        std::slice::from_raw_parts(int_ptr, (num_frames as usize) * channels);
                    int_samples.iter().map(|&s| s as f32 / 32768.0).collect()
                } else {
                    eprintln!(
                        "[Microphone] ERROR: Unsupported bits per sample: {}",
                        bits_per_sample
                    );
                    vec![]
                };

                // 释放缓冲区
                let _ = capture_client.ReleaseBuffer(num_frames);

                if samples.is_empty() {
                    continue;
                }

                // 检查是否是静音
                let is_silent = (flags & (AUDCLNT_BUFFERFLAGS_SILENT.0 as u32)) != 0;

                // 转换为单声道
                let mono_samples: Vec<f32> = if is_silent {
                    vec![0.0; num_frames as usize]
                } else {
                    samples
                        .chunks(channels)
                        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
                        .collect()
                };

                audio_buffer.extend(mono_samples);

                // 当缓冲区足够大时处理
                while audio_buffer.len() >= chunk_size {
                    let chunk: Vec<f32> = audio_buffer.drain(..chunk_size).collect();

                    let output = if let Some(ref resampler) = resampler {
                        let mut resampler_guard = resampler.lock().unwrap();
                        let frames_needed = resampler_guard.input_frames_next();
                        if chunk.len() < frames_needed {
                            continue;
                        }
                        match resampler_guard.process(&[chunk[..frames_needed].to_vec()], None) {
                            Ok(resampled) => resampled.into_iter().next().unwrap_or_default(),
                            Err(e) => {
                                eprintln!("[Microphone] Resampler error: {}", e);
                                continue;
                            }
                        }
                    } else {
                        chunk
                    };

                    if tx.send(output).is_err() {
                        eprintln!("[Microphone] ERROR: Failed to send audio data - channel closed");
                        break;
                    }
                }
            }
        }

        // 停止捕获
        let _ = audio_client.Stop();
        CoUninitialize();

        Ok(())
    }
}
