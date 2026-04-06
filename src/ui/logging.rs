use chrono::Local;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warning,
    Error,
    Success,
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

    fn lock_messages(&self) -> MutexGuard<'_, Vec<LogMessage>> {
        match self.messages.lock() {
            Ok(guard) => guard,
            Err(poisoned) => {
                eprintln!("Warning: Logger mutex was poisoned. Recovering...");
                poisoned.into_inner()
            }
        }
    }

    pub fn log(&self, level: LogLevel, message: impl AsRef<str>) {
        let mut messages = self.lock_messages();
        messages.push(LogMessage::new(level, message.as_ref().to_string()));
        if messages.len() > self.max_messages {
            let to_remove = messages.len() - self.max_messages;
            messages.drain(0..to_remove);
        }
    }

    pub fn info(&self, message: impl AsRef<str>) {
        self.log(LogLevel::Info, message);
    }

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
        self.lock_messages().clone()
    }

    pub fn clear(&self) {
        self.lock_messages().clear();
    }
}

const DEFAULT_MAX_MESSAGES: usize = 1000;
static GLOBAL_LOGGER: OnceLock<Arc<Logger>> = OnceLock::new();
static VERBOSE_MODE: AtomicBool = AtomicBool::new(false);
static TERMINAL_LOG_MODE: AtomicBool = AtomicBool::new(false);

pub fn set_verbose(verbose: bool) {
    VERBOSE_MODE.store(verbose, Ordering::SeqCst);
}

pub fn is_verbose() -> bool {
    VERBOSE_MODE.load(Ordering::SeqCst)
}

pub fn set_terminal_log(enabled: bool) {
    TERMINAL_LOG_MODE.store(enabled, Ordering::SeqCst);
}

pub fn is_terminal_log() -> bool {
    TERMINAL_LOG_MODE.load(Ordering::SeqCst)
}

/// Initializes the global logger with the given capacity.
/// The first call wins; subsequent calls are no-ops and return the existing logger.
pub fn init_logger(max_messages: usize) -> Arc<Logger> {
    GLOBAL_LOGGER
        .get_or_init(|| Arc::new(Logger::new(max_messages)))
        .clone()
}

pub fn get_logger() -> Arc<Logger> {
    GLOBAL_LOGGER
        .get_or_init(|| Arc::new(Logger::new(DEFAULT_MAX_MESSAGES)))
        .clone()
}

#[macro_export]
macro_rules! log_info {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        if $crate::ui::logging::is_terminal_log() {
            println!("[INFO] {}", message);
        }
        $crate::ui::logging::get_logger().info(message);
    }}
}

#[macro_export]
macro_rules! log_warning {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        if $crate::ui::logging::is_terminal_log() {
            println!("[WARN] {}", message);
        }
        $crate::ui::logging::get_logger().warning(message);
    }}
}

#[macro_export]
macro_rules! log_error {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        if $crate::ui::logging::is_terminal_log() {
            eprintln!("[ERROR] {}", message);
        }
        $crate::ui::logging::get_logger().error(message);
    }}
}

#[macro_export]
macro_rules! log_success {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        if $crate::ui::logging::is_terminal_log() {
            println!("[OK] {}", message);
        }
        $crate::ui::logging::get_logger().success(message);
    }}
}

#[macro_export]
macro_rules! log_progress {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        if $crate::ui::logging::is_terminal_log() {
            println!("[...] {}", message);
        }
        $crate::ui::logging::get_logger().progress(message);
    }}
}

#[macro_export]
macro_rules! log_verbose {
    ($($arg:tt)*) => {{
        if $crate::ui::logging::is_verbose() {
            let message = format!($($arg)*);
            if $crate::ui::logging::is_terminal_log() {
                println!("[VERBOSE] {}", message);
            }
            $crate::ui::logging::get_logger().info(message);
        }
    }}
}
