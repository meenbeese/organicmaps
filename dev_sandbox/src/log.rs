use std::ffi::c_char;

use crate::c_api;

/// Forwards a message to the C++ logging subsystem.
pub fn message(level: i32, msg: &str) {
    unsafe {
        c_api::om_log_message(level, msg.as_ptr() as *const c_char, msg.len());
    }
}

pub fn info(msg: &str) {
    message(c_api::LOG_INFO, msg);
}

pub fn warning(msg: &str) {
    message(c_api::LOG_WARNING, msg);
}

pub fn error(msg: &str) {
    message(c_api::LOG_ERROR, msg);
}
