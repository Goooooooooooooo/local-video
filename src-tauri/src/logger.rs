// 导入必要的库
use std::fs::{File, OpenOptions};      // 文件操作
use std::io::Write;                    // 写入文件
use std::path::PathBuf;                // 路径处理
use std::sync::Mutex;                  // 线程同步
use std::sync::atomic::{AtomicU8, Ordering};

// 使用 once_cell 替代 lazy_static（once_cell 已进入标准库）
use std::sync::OnceLock;
use chrono::Local;

static LOGGER: OnceLock<Mutex<Logger>> = OnceLock::new();

// 获取当前日期字符串
fn get_current_date() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

// 获取当前时间字符串
fn get_current_time() -> String {
    Local::now().format("%Y-%m-%d %H:%M:%S.%3f").to_string()
}

// 日志器结构体
pub struct Logger {
    current_date: String,              // 当前日期，用于检查是否需要新建日志文件
    log_file: Option<File>,            // 当前日志文件句柄
    log_dir: PathBuf,                  // 日志目录路径
}

// 定义日志级别
#[derive(PartialEq, PartialOrd)]
pub enum LogLevel {
    ERROR = 0,
    INFO = 1,
    DEBUG = 2,
}

// 全局日志级别
static LOG_LEVEL: AtomicU8 = AtomicU8::new(LogLevel::INFO as u8);

impl Logger {
    // 创建新的日志器实例
    fn new() -> Self {
        // 获取可执行文件所在目录下的 logs 文件夹
        let log_dir = std::env::current_exe()
            .unwrap_or_default()
            .parent()
            .unwrap_or(&std::path::Path::new("."))
            .join("logs");
        println!("log_dir: {}", log_dir.to_string_lossy());
        // 确保日志目录存在
        std::fs::create_dir_all(&log_dir).unwrap_or_default();

        Logger {
            current_date: get_current_date(),
            log_file: None,
            log_dir,
        }
    }

    // 确保日志文件存在并是当前日期的
    fn ensure_log_file(&mut self) -> std::io::Result<()> {
        let today = get_current_date();
        
        // 如果日期变化或文件未打开，创建新文件
        if self.current_date != today || self.log_file.is_none() {
            self.current_date = today.clone();
            let log_path = self.log_dir.join(format!("{}.log", today));
            
            // 打开或创建日志文件，设置为追加模式
            self.log_file = Some(OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)?);
        }
        Ok(())
    }

    // 设置日志级别
    pub fn set_log_level(level: LogLevel) {
        LOG_LEVEL.store(level as u8, Ordering::SeqCst);
    }

    fn match_log_level(level: LogLevel) -> bool {
        let current_level = LOG_LEVEL.load(Ordering::SeqCst);
        (level as u8) <= current_level
    }

    // 写入日志
    fn write(&mut self, level: &str, message: &str) -> std::io::Result<()> {
        self.ensure_log_file()?;

        let log_level = match level {
            "ERROR" => LogLevel::ERROR,
            "INFO" => LogLevel::INFO,
            "DEBUG" => LogLevel::DEBUG,
            _ => LogLevel::INFO,
        };

        if !Self::match_log_level(log_level) {
            return Ok(());
        }
        
        if let Some(file) = &mut self.log_file {
            // 只格式化日志的元数据部分，保持消息文本原样
            let timestamp = get_current_time();
            let thread_name = std::thread::current().name().unwrap_or("unknown").to_string();
            let log_message = format!("[{timestamp}] [{level}] {thread_name} - {message}\n");
            
            file.write_all(log_message.as_bytes())?;  // 写入文件
            file.flush()?;                            // 立即刷新到磁盘
        }
        Ok(())
    }
}

// 初始化日志器
pub fn init_logger() -> Result<(), String> {
    LOGGER.set(Mutex::new(Logger::new()))
        .map_err(|_| "Logger already initialized".to_string())
}

// 公共设置接口
pub fn set_log_level(level: LogLevel) {
    Logger::set_log_level(level);
}

// 公共日志接口函数
pub fn log_error(message: &str) {
    if let Some(logger) = LOGGER.get() {
        if let Ok(mut guard) = logger.lock() {
            guard.write("ERROR", message).unwrap_or_default();
        }
    }
}

pub fn log_info(message: &str) {
    if let Some(logger) = LOGGER.get() {
        if let Ok(mut guard) = logger.lock() {
            guard.write("INFO", message).unwrap_or_default();
        }
    }
}

pub fn log_debug(message: &str) {
    if let Some(logger) = LOGGER.get() {
        if let Ok(mut guard) = logger.lock() {
            guard.write("DEBUG", message).unwrap_or_default();
        }
    }
}

// 便捷宏定义
#[macro_export]  // 导出宏，使其在其他模块可用
macro_rules! log_error {
    ($($arg:tt)*) => ({
        // 使用 format! 宏处理格式化字符串
        $crate::logger::log_error(&format!($($arg)*));
    })
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => ({
        $crate::logger::log_info(&format!($($arg)*));
    })
}

#[macro_export]
macro_rules! log_debug {
    ($($arg:tt)*) => ({
        $crate::logger::log_debug(&format!($($arg)*));
    })
}
