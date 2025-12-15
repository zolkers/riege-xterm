use std::sync::{Mutex, OnceLock};
use crate::core::ui::MessageLogger;

pub static GLOBAL_LOGGER: OnceLock<Mutex<Option<MessageLogger>>> = OnceLock::new();

pub fn set_logger(logger: MessageLogger) {
    let lock = GLOBAL_LOGGER.get_or_init(|| Mutex::new(None));
    let mut global = lock.lock().unwrap();
    *global = Some(logger);
}

fn with_logger<F>(f: F)
where F: FnOnce(&MessageLogger)
{
    if let Some(lock) = GLOBAL_LOGGER.get() {
        if let Ok(global) = lock.lock() {
            if let Some(logger) = global.as_ref() {
                f(logger);
            }
        }
    }
}

pub fn log(message: String) {
    with_logger(|l| l.log(message));
}

pub fn print_line(message: &str) {
    log(message.to_string());
}

pub fn info(message: &str) {
    with_logger(|l| l.info(message));
}

pub fn error(message: &str) {
    with_logger(|l| l.error(message));
}

pub fn success(message: &str) {
    with_logger(|l| l.success(message));
}

pub fn warning(message: &str) {
    with_logger(|l| l.warning(message));
}

pub fn debug(message: &str) {
    with_logger(|l| l.debug(message));
}