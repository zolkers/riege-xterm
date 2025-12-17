use crate::core::ui::{MessageLogger, TerminalUI};
use std::sync::{Mutex, OnceLock};
use std::sync::atomic::{AtomicBool, Ordering};

pub static SHUTDOWN_SIGNAL: AtomicBool = AtomicBool::new(false);
pub static JAVA_INPUT_CALLBACK: OnceLock<Box<dyn Fn(&str) + Send + Sync>> = OnceLock::new();
pub static JAVA_TAB_CALLBACK: OnceLock<Box<dyn Fn(&str) + Send + Sync>> = OnceLock::new();
pub static COMPLETION_CANDIDATES: Mutex<Vec<String>> = Mutex::new(Vec::new());

pub struct Terminal {}

impl Terminal {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn run(&mut self) -> anyhow::Result<()> {
        eprintln!("[RUST DEBUG] Terminal::run() starting");
        let mut ui = TerminalUI::new();
        eprintln!("[RUST DEBUG] TerminalUI created");
        let logger = ui.get_message_logger();

        crate::core::logger::set_logger(logger.clone());
        eprintln!("[RUST DEBUG] Logger set");

        self.add_banner(&logger);
        eprintln!("[RUST DEBUG] Banner added");
        ui.set_prompt("rmc > ".to_string());
        eprintln!("[RUST DEBUG] Prompt set, calling ui.run()");

        ui.run(
            move |raw_input| {
                async move {
                    if SHUTDOWN_SIGNAL.load(Ordering::Relaxed) {
                        return Ok(true);
                    }
                    if let Some(callback) = JAVA_INPUT_CALLBACK.get() {
                        callback(raw_input.trim());
                    } else {
                        crate::core::logger::error("Backend disconnected.");
                    }
                    Ok(false)
                }
            },
            move |current_buffer, _cursor_pos| {
                if let Ok(mut candidates) = COMPLETION_CANDIDATES.lock() {
                    candidates.clear();
                }

                if let Some(callback) = JAVA_TAB_CALLBACK.get() {
                    callback(current_buffer);
                }

                if let Ok(candidates) = COMPLETION_CANDIDATES.lock() {
                    candidates.clone()
                } else {
                    Vec::new()
                }
            }
        ).await?;

        eprintln!("[RUST DEBUG] ui.run() completed");
        Ok(())
    }

    fn add_banner(&self, logger: &MessageLogger) {
        logger.log("[RUST1] ██████╗ ██╗███████╗ ██████╗ ███████╗".to_string());
        logger.log("[RUST2] ██╔══██╗██║██╔════╝██╔════╝ ██╔════╝".to_string());
        logger.log("[RUST3] ██████╔╝██║█████╗  ██║  ███╗█████╗  ".to_string());
        logger.log("[RUST4] ██╔══██╗██║██╔══╝  ██║   ██║██╔══╝  ".to_string());
        logger.log("[RUST5] ██║  ██║██║███████╗╚██████╔╝███████╗".to_string());
        logger.log("[RUST6] ╚═╝  ╚═╝╚═╝╚══════╝ ╚═════╝ ╚══════╝".to_string());
        logger.log("".to_string());
    }
}