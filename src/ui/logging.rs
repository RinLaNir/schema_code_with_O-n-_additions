use std::sync::{Arc, Mutex};
use chrono::Local;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    #[allow(dead_code)]
    Warning,
    Error,
    Success,
    #[allow(dead_code)]
    Progress,
} 

#[derive(Debug, Clone)]
pub struct LogMessage {
    pub timestamp: chrono::DateTime<Local>,
    pub level: LogLevel,
    pub message: String,
}

impl LogMessage {
    pub fn new(level: LogLevel, message: String) -> Self {
        Self {
            timestamp: Local::now(),
            level,
            message,
        }
    }

    pub fn formatted_timestamp(&self) -> String {
        self.timestamp.format("%H:%M:%S").to_string()
    }
}

pub struct Logger {
    messages: Arc<Mutex<Vec<LogMessage>>>,
    max_messages: usize,
} 

impl Logger {
    pub fn new(max_messages: usize) -> Self {
        Self {
            messages: Arc::new(Mutex::new(Vec::new())),
            max_messages,
        }
    }

    pub fn log(&self, level: LogLevel, message: impl AsRef<str>) {
        {
            let mut messages = match self.messages.lock() {
                Ok(guard) => guard,
                Err(poisoned) => {
                    eprintln!("Warning: Logger mutex was poisoned. Recovering...");
                    poisoned.into_inner()
                }
            };
            
            messages.push(LogMessage::new(level, message.as_ref().to_string()));
            
            if messages.len() > self.max_messages {
                let to_remove = messages.len() - self.max_messages;
                messages.drain(0..to_remove);
            }
        }
    }

    pub fn info(&self, message: impl AsRef<str>) {
        self.log(LogLevel::Info, message);
    }

    #[allow(dead_code)]
    pub fn warning(&self, message: impl AsRef<str>) {
        self.log(LogLevel::Warning, message);
    }

    pub fn error(&self, message: impl AsRef<str>) {
        self.log(LogLevel::Error, message);
    }

    pub fn success(&self, message: impl AsRef<str>) {
        self.log(LogLevel::Success, message);
    }

    #[allow(dead_code)]
    pub fn progress(&self, message: impl AsRef<str>) {
        self.log(LogLevel::Progress, message);
    }

    pub fn get_messages(&self) -> Vec<LogMessage> {
        match self.messages.lock() {
            Ok(guard) => guard.clone(),
            Err(poisoned) => {
                eprintln!("Warning: Logger mutex was poisoned when getting messages. Recovering...");
                poisoned.into_inner().clone()
            }
        }
    }

    pub fn clear(&self) {
        if let Ok(mut messages) = self.messages.lock() {
            messages.clear();
        } else {
            eprintln!("Warning: Failed to clear logger due to poisoned mutex");
        }
    }
}

lazy_static::lazy_static! {
    static ref GLOBAL_LOGGER: Mutex<Option<Arc<Logger>>> = Mutex::new(None);
} 

use std::sync::atomic::{AtomicBool, Ordering};
static VERBOSE_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_verbose(verbose: bool) {
    VERBOSE_MODE.store(verbose, Ordering::SeqCst);
}

pub fn is_verbose() -> bool {
    VERBOSE_MODE.load(Ordering::SeqCst)
} 

pub fn init_logger(max_messages: usize) -> Arc<Logger> {
    let logger = Arc::new(Logger::new(max_messages));
    
    if let Ok(mut global) = GLOBAL_LOGGER.lock() {
        *global = Some(logger.clone());
    } else {
        eprintln!("Warning: Failed to initialize global logger due to lock poisoning");
    }
    
    logger
} 

pub fn get_logger() -> Arc<Logger> {
    if let Ok(global) = GLOBAL_LOGGER.lock() {
        if let Some(logger) = &*global {
            return logger.clone();
        }
    }
    
    if let Ok(mut global) = GLOBAL_LOGGER.lock() {
        if let Some(logger) = &*global {
            return logger.clone();
        } else {
            let logger = Arc::new(Logger::new(1000));
            *global = Some(logger.clone());
            return logger;
        }
    } else {
        eprintln!("Warning: Failed to get/initialize global logger due to lock poisoning. Creating temporary logger.");
        Arc::new(Logger::new(1000))
    }
} 

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        $crate::ui::logging::get_logger().info(message);
    }}
}

#[macro_export]
macro_rules! log_warning {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        $crate::ui::logging::get_logger().warning(message);
    }}
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        $crate::ui::logging::get_logger().error(message);
    }}
}

#[macro_export]
macro_rules! log_success {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        $crate::ui::logging::get_logger().success(message);
    }}
}

#[macro_export]
macro_rules! log_progress {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        $crate::ui::logging::get_logger().progress(message);
    }}
}

#[macro_export]
macro_rules! log_verbose {
    ($($arg:tt)*) => {{
        if $crate::ui::logging::is_verbose() {
            let message = format!($($arg)*);
            $crate::ui::logging::get_logger().info(message);
        }
    }}
}