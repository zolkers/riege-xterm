use std::ffi::{CStr, CString};
use std::os::raw::{c_char};
use std::sync::atomic::Ordering;
use crate::core::logger;
use crate::core::repl_new::{
    SHUTDOWN_SIGNAL,
    JAVA_INPUT_CALLBACK,
    JAVA_TAB_CALLBACK,
    COMPLETION_CANDIDATES,
    Terminal
};

#[no_mangle]
pub extern "C" fn terminal_log_info(msg: *const c_char) {
    if msg.is_null() { return; }
    unsafe {
        if let Ok(c_str) = CStr::from_ptr(msg).to_str() {
            logger::info(c_str);
        }
    }
}

#[no_mangle]
pub extern "C" fn terminal_log_error(msg: *const c_char) {
    if msg.is_null() { return; }
    unsafe {
        if let Ok(c_str) = CStr::from_ptr(msg).to_str() {
            logger::error(c_str);
        }
    }
}

#[no_mangle]
pub extern "C" fn terminal_log_success(msg: *const c_char) {
    if msg.is_null() { return; }
    unsafe {
        if let Ok(c_str) = CStr::from_ptr(msg).to_str() {
            logger::success(c_str);
        }
    }
}

#[no_mangle]
pub extern "C" fn terminal_close() {
    SHUTDOWN_SIGNAL.store(true, Ordering::Relaxed);
}

#[no_mangle]
pub extern "C" fn terminal_add_candidate(candidate: *const c_char) {
    if candidate.is_null() { return; }
    unsafe {
        if let Ok(c_str) = CStr::from_ptr(candidate).to_str() {
            if let Ok(mut list) = COMPLETION_CANDIDATES.lock() {
                list.push(c_str.to_string());
            }
        }
    }
}

pub type NativeCallback = extern "C" fn(*const c_char);
static mut RAW_INPUT_CB: Option<NativeCallback> = None;
static mut RAW_TAB_CB: Option<NativeCallback> = None;

fn invoke_native_callback(cb_opt: Option<NativeCallback>, data: &str) {
    if let Some(cb) = cb_opt {
        if let Ok(c_string) = CString::new(data) {
            unsafe {
                cb(c_string.as_ptr());
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn terminal_register_input_callback(callback: NativeCallback) {
    unsafe { RAW_INPUT_CB = Some(callback); }

    JAVA_INPUT_CALLBACK.get_or_init(|| {
        Box::new(move |input| {
            unsafe { invoke_native_callback(RAW_INPUT_CB, input); }
        })
    });
}

#[no_mangle]
pub extern "C" fn terminal_register_tab_callback(callback: NativeCallback) {
    unsafe { RAW_TAB_CB = Some(callback); }

    JAVA_TAB_CALLBACK.get_or_init(|| {
        Box::new(move |buffer| {
            unsafe { invoke_native_callback(RAW_TAB_CB, buffer); }
        })
    });
}

#[no_mangle]
pub extern "C" fn terminal_start() {
    let runtime = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");

    runtime.block_on(async {
        let mut terminal = Terminal::new();
        if let Err(e) = terminal.run().await {
            logger::error(&format!("Terminal error: {}", e));
        }
    });
}