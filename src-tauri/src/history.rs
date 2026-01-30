// 历史识别记录管理模块
// 每次识别会话的结果会保存为本地 txt 文件

use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use chrono::{DateTime, Local, TimeZone};

/// 单条字幕记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtitleRecord {
    /// 字幕文本
    pub text: String,
    /// 时间戳（毫秒）
    pub timestamp: u64,
}

/// 历史记录会话
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistorySession {
    /// 会话 ID（文件名，格式：YYYY-MM-DD_HH-mm-ss）
    pub id: String,
    /// 开始时间（毫秒时间戳）
    pub start_time: u64,
    /// 结束时间（毫秒时间戳）
    pub end_time: u64,
    /// 字幕记录列表
    pub records: Vec<SubtitleRecord>,
}

impl HistorySession {
    /// 创建新会话
    pub fn new() -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let local: DateTime<Local> = Local.timestamp_millis_opt(now as i64).unwrap();
        let id = local.format("%Y-%m-%d_%H-%M-%S").to_string();
        
        Self {
            id,
            start_time: now,
            end_time: now,
            records: Vec::new(),
        }
    }

    /// 添加字幕记录
    pub fn add_record(&mut self, text: String, timestamp: u64) {
        self.records.push(SubtitleRecord { text, timestamp });
        self.end_time = timestamp;
    }

    /// 获取会话时长（秒）
    pub fn duration_secs(&self) -> u64 {
        (self.end_time - self.start_time) / 1000
    }

    /// 获取文本预览（前100个字符）
    pub fn preview(&self) -> String {
        let full_text: String = self.records.iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
            .join(" ");
        
        // 使用 chars() 按字符而非字节截取，避免 UTF-8 边界问题
        let char_count = full_text.chars().count();
        if char_count > 100 {
            let preview: String = full_text.chars().take(100).collect();
            format!("{}...", preview)
        } else {
            full_text
        }
    }

    /// 获取完整文本
    pub fn full_text(&self) -> String {
        self.records.iter()
            .map(|r| r.text.as_str())
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// 保存到文件
    pub fn save_to_file(&self, dir: &PathBuf) -> Result<PathBuf, String> {
        // 确保目录存在
        fs::create_dir_all(dir).map_err(|e| format!("创建目录失败: {}", e))?;

        let file_path = dir.join(format!("{}.txt", self.id));
        let mut file = fs::File::create(&file_path)
            .map_err(|e| format!("创建文件失败: {}", e))?;

        // 写入元数据头
        writeln!(file, "# Live Subtitles 历史记录").map_err(|e| e.to_string())?;
        writeln!(file, "# 开始时间: {}", format_timestamp(self.start_time)).map_err(|e| e.to_string())?;
        writeln!(file, "# 结束时间: {}", format_timestamp(self.end_time)).map_err(|e| e.to_string())?;
        writeln!(file, "# 时长: {}秒", self.duration_secs()).map_err(|e| e.to_string())?;
        writeln!(file, "# 字幕条数: {}", self.records.len()).map_err(|e| e.to_string())?;
        writeln!(file, "").map_err(|e| e.to_string())?;

        // 写入字幕内容
        for record in &self.records {
            let time_str = format_timestamp(record.timestamp);
            writeln!(file, "[{}] {}", time_str, record.text).map_err(|e| e.to_string())?;
        }

        Ok(file_path)
    }

    /// 从文件加载
    pub fn load_from_file(file_path: &PathBuf) -> Result<Self, String> {
        let file = fs::File::open(file_path)
            .map_err(|e| format!("打开文件失败: {}", e))?;
        let reader = BufReader::new(file);

        let mut start_time: u64 = 0;
        let mut end_time: u64 = 0;
        let mut records = Vec::new();

        // 从文件名获取 ID
        let id = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        for line in reader.lines() {
            let line = line.map_err(|e| e.to_string())?;
            
            // 解析元数据 - 使用字符串匹配而非字节索引，避免中文字符边界问题
            let start_prefix = "# 开始时间: ";
            let end_prefix = "# 结束时间: ";
            
            if line.starts_with(start_prefix) {
                if let Some(ts) = parse_timestamp(&line[start_prefix.len()..]) {
                    start_time = ts;
                }
            } else if line.starts_with(end_prefix) {
                if let Some(ts) = parse_timestamp(&line[end_prefix.len()..]) {
                    end_time = ts;
                }
            } else if line.starts_with('[') {
                // 解析字幕行: [2026-01-30 14:30:00] 字幕内容
                if let Some(end_bracket) = line.find(']') {
                    let time_str = &line[1..end_bracket];
                    let text = line[end_bracket + 2..].to_string();
                    
                    if let Some(timestamp) = parse_timestamp(time_str) {
                        records.push(SubtitleRecord { text, timestamp });
                    }
                }
            }
        }

        Ok(Self {
            id,
            start_time,
            end_time,
            records,
        })
    }
}

/// 历史记录列表项（用于列表展示，不包含完整内容）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryListItem {
    /// 会话 ID
    pub id: String,
    /// 开始时间（毫秒时间戳）
    pub start_time: u64,
    /// 结束时间（毫秒时间戳）
    pub end_time: u64,
    /// 时长（秒）
    pub duration_secs: u64,
    /// 字幕条数
    pub record_count: usize,
    /// 文本预览
    pub preview: String,
    /// 文件路径
    pub file_path: String,
}

/// 历史记录管理器
pub struct HistoryManager {
    /// 存储目录
    history_dir: PathBuf,
    /// 当前会话
    current_session: Option<HistorySession>,
}

impl HistoryManager {
    /// 创建管理器
    pub fn new(history_dir: PathBuf) -> Self {
        Self {
            history_dir,
            current_session: None,
        }
    }

    /// 设置存储目录
    pub fn set_history_dir(&mut self, dir: PathBuf) {
        self.history_dir = dir;
    }

    /// 获取存储目录
    pub fn get_history_dir(&self) -> &PathBuf {
        &self.history_dir
    }

    /// 开始新会话
    pub fn start_session(&mut self) {
        self.current_session = Some(HistorySession::new());
    }

    /// 添加字幕到当前会话
    pub fn add_subtitle(&mut self, text: String, timestamp: u64) {
        if let Some(session) = &mut self.current_session {
            session.add_record(text, timestamp);
        }
    }

    /// 保存当前会话（不结束会话，用于定期保存）
    pub fn save_current_session(&mut self) -> Option<PathBuf> {
        if let Some(session) = &mut self.current_session {
            // 只有当有记录时才保存
            if !session.records.is_empty() {
                // 更新结束时间为当前时间
                session.end_time = chrono::Local::now().timestamp_millis() as u64;
                match session.save_to_file(&self.history_dir) {
                    Ok(path) => {
                        return Some(path);
                    }
                    Err(e) => {
                        eprintln!("保存历史记录失败: {}", e);
                    }
                }
            }
        }
        None
    }

    /// 结束并保存当前会话
    pub fn end_session(&mut self) -> Option<PathBuf> {
        if let Some(session) = self.current_session.take() {
            // 只有当有记录时才保存
            if !session.records.is_empty() {
                match session.save_to_file(&self.history_dir) {
                    Ok(path) => {
                        println!("历史记录已保存: {:?}", path);
                        return Some(path);
                    }
                    Err(e) => {
                        eprintln!("保存历史记录失败: {}", e);
                    }
                }
            }
        }
        None
    }

    /// 获取历史记录列表
    pub fn list_history(&self) -> Result<Vec<HistoryListItem>, String> {
        if !self.history_dir.exists() {
            return Ok(Vec::new());
        }

        let mut items = Vec::new();
        let entries = fs::read_dir(&self.history_dir)
            .map_err(|e| format!("读取目录失败: {}", e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("txt") {
                if let Ok(session) = HistorySession::load_from_file(&path) {
                    items.push(HistoryListItem {
                        id: session.id.clone(),
                        start_time: session.start_time,
                        end_time: session.end_time,
                        duration_secs: session.duration_secs(),
                        record_count: session.records.len(),
                        preview: session.preview(),
                        file_path: path.to_string_lossy().to_string(),
                    });
                }
            }
        }

        // 按开始时间倒序排列
        items.sort_by(|a, b| b.start_time.cmp(&a.start_time));

        Ok(items)
    }

    /// 获取历史记录详情
    pub fn get_history_detail(&self, id: &str) -> Result<HistorySession, String> {
        let file_path = self.history_dir.join(format!("{}.txt", id));
        HistorySession::load_from_file(&file_path)
    }

    /// 删除历史记录
    pub fn delete_history(&self, id: &str) -> Result<(), String> {
        let file_path = self.history_dir.join(format!("{}.txt", id));
        fs::remove_file(&file_path)
            .map_err(|e| format!("删除文件失败: {}", e))
    }

    /// 搜索历史记录
    pub fn search_history(&self, keyword: &str) -> Result<Vec<HistoryListItem>, String> {
        let all_items = self.list_history()?;
        let keyword_lower = keyword.to_lowercase();
        
        let filtered: Vec<HistoryListItem> = all_items
            .into_iter()
            .filter(|item| {
                // 在预览中搜索
                if item.preview.to_lowercase().contains(&keyword_lower) {
                    return true;
                }
                // 在 ID（日期）中搜索
                if item.id.contains(&keyword_lower) {
                    return true;
                }
                // 需要完整内容搜索时，加载文件
                if let Ok(session) = self.get_history_detail(&item.id) {
                    return session.full_text().to_lowercase().contains(&keyword_lower);
                }
                false
            })
            .collect();

        Ok(filtered)
    }

    /// 清空所有历史记录
    pub fn clear_all(&self) -> Result<usize, String> {
        if !self.history_dir.exists() {
            return Ok(0);
        }

        let mut count = 0;
        let entries = fs::read_dir(&self.history_dir)
            .map_err(|e| format!("读取目录失败: {}", e))?;

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("txt") {
                if fs::remove_file(&path).is_ok() {
                    count += 1;
                }
            }
        }

        Ok(count)
    }
}

/// 格式化时间戳为字符串
fn format_timestamp(timestamp: u64) -> String {
    let local: DateTime<Local> = Local.timestamp_millis_opt(timestamp as i64).unwrap();
    local.format("%Y-%m-%d %H:%M:%S").to_string()
}

/// 解析时间字符串为时间戳
fn parse_timestamp(time_str: &str) -> Option<u64> {
    // 尝试解析格式: 2026-01-30 14:30:00
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(time_str, "%Y-%m-%d %H:%M:%S") {
        let local = Local.from_local_datetime(&dt).single()?;
        return Some(local.timestamp_millis() as u64);
    }
    None
}
