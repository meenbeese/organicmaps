//! Rust shell for the Organic Maps developer sandbox.
//!
//! The C++ core (map::Framework, drape, storage) is reached through the C ABI
//! in `dev_sandbox/cshim/c_api.h`. This crate is built as a static library and
//! linked by CMake together with the C++ core, which also compiles the C ABI
//! shim (`cshim/c_api.cpp`). The Rust shell provides the executable's `main`,
//! so no C++ `main` is needed.
//!
//! `unsafe fn` is this crate's trusted FFI boundary: every one wraps C++ calls
//! behind `c_api.h`. The bodies are trusted, so edition 2024's requirement to
//! re-assert each call inside `unsafe {}` adds nothing.
#![allow(unsafe_op_in_unsafe_fn)]

pub mod app;
pub mod c_api;
pub mod glfw;
pub mod imgui;
pub mod log;
pub mod mercator;
pub mod sandbox;
pub mod ui;

use std::ffi::c_char;
use std::ffi::c_int;

/// Executable entry point. Arguments are ignored: the platform finds the
/// resources directory by itself, see `platform_linux.cpp::Platform`.
///
/// The crate is a `staticlib`, so this `main` must be exported manually;
/// glibc's `__libc_start_main` calls it directly. `std` features used here
/// (threads, TLS, atomics, time) do not need `std::rt::lang_start`.
#[unsafe(no_mangle)]
pub extern "C" fn main(_argc: c_int, _argv: *mut *mut c_char) -> c_int {
    match app::run() {
        Ok(()) => 0,
        Err(err) => {
            eprintln!("dev_sandbox: {err}");
            1
        }
    }
}
