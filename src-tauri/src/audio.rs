//! 音频捕获模块
//! 使用 cpal 实现 WASAPI loopback 捕获系统音频

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Host, Sample, Stream, StreamConfig};
use rubato::{FftFixedInOut, Resampler};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};

/// 音频捕获器
pub struct AudioCapture {
    stream: Option<Stream>,
    sample_rate: u32,
    target_sample_rate: u32,
}

/// 音频数据接收器
pub type AudioReceiver = Receiver<Vec<f32>>;

impl AudioCapture {
    /// 创建音频捕获器
    ///
    /// # Arguments
    /// * `target_sample_rate` - 目标采样率 (通常为 16000)
    pub fn new(target_sample_rate: u32) -> Self {
        Self {
            stream: None,
            sample_rate: 0,
            target_sample_rate,
        }
    }

    /// 获取默认的 loopback 设备
    #[cfg(target_os = "windows")]
    fn get_loopback_device() -> Result<(Host, Device, StreamConfig), String> {
        use cpal::SupportedStreamConfig;

        let host = cpal::host_from_id(cpal::HostId::Wasapi)
            .map_err(|e| format!("Failed to get WASAPI host: {}", e))?;

        // 获取默认输出设备
        let device = host
            .default_output_device()
            .ok_or_else(|| "No default output device found".to_string())?;

        println!("Using output device for loopback: {:?}", device.name());

        // 获取输出设备的配置
        let supported_config: SupportedStreamConfig = device
            .default_output_config()
            .map_err(|e| format!("Failed to get output config: {}", e))?;

        let config: StreamConfig = supported_config.into();
        println!("Audio config: {:?}", config);

        Ok((host, device, config))
    }

    #[cfg(not(target_os = "windows"))]
    fn get_loopback_device() -> Result<(Host, Device, StreamConfig), String> {
        Err("Loopback capture is only supported on Windows".to_string())
    }

    /// 开始捕获音频
    ///
    /// 返回一个接收器用于获取音频数据
    pub fn start(&mut self) -> Result<AudioReceiver, String> {
        let (_host, device, config) = Self::get_loopback_device()?;

        self.sample_rate = config.sample_rate.0;
        let channels = config.channels as usize;

        println!(
            "Source sample rate: {}, channels: {}",
            self.sample_rate, channels
        );

        // 创建通道用于传输音频数据
        let (tx, rx): (Sender<Vec<f32>>, Receiver<Vec<f32>>) = mpsc::channel();

        // 创建重采样器 (如果需要)
        let resampler = if self.sample_rate != self.target_sample_rate {
            Some(Arc::new(Mutex::new(
                FftFixedInOut::<f32>::new(
                    self.sample_rate as usize,
                    self.target_sample_rate as usize,
                    1024, // chunk size
                    1,    // mono
                )
                .map_err(|e| format!("Failed to create resampler: {}", e))?,
            )))
        } else {
            None
        };

        // 音频缓冲区
        let buffer = Arc::new(Mutex::new(Vec::<f32>::new()));
        let buffer_clone = buffer.clone();
        let resampler_clone = resampler.clone();

        // 创建流 (使用 F32 格式)
        let stream = self.build_stream::<f32>(
            &device,
            &config,
            channels,
            tx,
            buffer_clone,
            resampler_clone,
        )?;

        stream
            .play()
            .map_err(|e| format!("Failed to play stream: {}", e))?;
        self.stream = Some(stream);

        Ok(rx)
    }

    /// 构建音频流
    fn build_stream<T>(
        &self,
        device: &Device,
        config: &StreamConfig,
        channels: usize,
        tx: Sender<Vec<f32>>,
        buffer: Arc<Mutex<Vec<f32>>>,
        resampler: Option<Arc<Mutex<FftFixedInOut<f32>>>>,
    ) -> Result<Stream, String>
    where
        T: cpal::Sample + cpal::SizedSample + Send + 'static,
        f32: cpal::FromSample<T>,
    {
        let chunk_size = 1024;

        let stream = device
            .build_input_stream(
                config,
                move |data: &[T], _: &cpal::InputCallbackInfo| {
                    // 转换为 f32 并混合为单声道
                    let mono_samples: Vec<f32> = data
                        .chunks(channels)
                        .map(|frame| {
                            let sum: f32 = frame.iter().map(|&s| f32::from_sample(s)).sum();
                            sum / channels as f32
                        })
                        .collect();

                    // 添加到缓冲区
                    let mut buf = buffer.lock().unwrap();
                    buf.extend(mono_samples);

                    // 当缓冲区足够大时处理
                    while buf.len() >= chunk_size {
                        let chunk: Vec<f32> = buf.drain(..chunk_size).collect();

                        let output = if let Some(ref resampler) = resampler {
                            // 重采样
                            let mut resampler = resampler.lock().unwrap();
                            match resampler.process(&[chunk], None) {
                                Ok(resampled) => resampled.into_iter().next().unwrap_or_default(),
                                Err(_) => continue,
                            }
                        } else {
                            chunk
                        };

                        let _ = tx.send(output);
                    }
                },
                |err| {
                    eprintln!("Audio capture error: {}", err);
                },
                None,
            )
            .map_err(|e| format!("Failed to build input stream: {}", e))?;

        Ok(stream)
    }

    /// 停止捕获
    pub fn stop(&mut self) {
        self.stream = None;
    }

    /// 是否正在捕获
    pub fn is_capturing(&self) -> bool {
        self.stream.is_some()
    }
}

impl Drop for AudioCapture {
    fn drop(&mut self) {
        self.stop();
    }
}
