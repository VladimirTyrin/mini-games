use std::sync::OnceLock;
use chrono::Local;

static LOGGER: OnceLock<Logger> = OnceLock::new();

pub struct Logger {
    prefix: Option<String>,
}

impl Logger {
    fn new(prefix: Option<String>) -> Self {
        Self { prefix }
    }

    pub fn log(&self, file: &str, line: u32, message: &str) {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        let file_name = file.rsplit(['/', '\\']).next().unwrap_or(file);
        if let Some(ref prefix) = self.prefix {
            println!("[{}][{}][{}:{}] {}", timestamp, prefix, file_name, line, message);
        } else {
            println!("[{}][{}:{}] {}", timestamp, file_name, line, message);
        }
    }
}

pub fn init_logger(prefix: Option<String>) {
    LOGGER.get_or_init(|| Logger::new(prefix));
}

pub fn log(file: &str, line: u32, message: &str) {
    if let Some(logger) = LOGGER.get() {
        logger.log(file, line, message);
    } else {
        eprintln!("Logger not initialized! Call init_logger() first.");
    }
}

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {
        $crate::logger::log(file!(), line!(), &format!($($arg)*))
    };
}
