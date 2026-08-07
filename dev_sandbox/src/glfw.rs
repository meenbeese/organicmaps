//! Thin wrapper around the `glfw` crate.
//!
//! The crate (via glfw-sys) builds and owns the project's single GLFW
//! instance; the C++ core reaches the very same `GLFWwindow` through the
//! opaque pointer handed to it by `Window::raw_void`. Only the small subset
//! used by the sandbox is exposed, and everything runs on the main thread.

use std::cell::RefCell;
use std::ffi::c_int;
use std::ffi::c_void;

use glfw::{Action, Key, Modifiers, MouseButton, WindowMode};

pub use glfw::{ClientApiHint, WindowHint};

// GLFW input constants (raw C values, as used by the input handlers).
pub const GLFW_MOUSE_BUTTON_LEFT: c_int = 0;
pub const GLFW_PRESS: c_int = 1;
pub const GLFW_RELEASE: c_int = 0;
pub const GLFW_MOD_SUPER: c_int = 0x0008;

thread_local! {
  static GLFW: RefCell<Option<glfw::Glfw>> = const { RefCell::new(None) };
}

pub struct VidMode {
    pub width: c_int,
    pub height: c_int,
}

/// Initializes GLFW and installs the error logger.
pub fn init() -> bool {
    let glfw = glfw::init(|err, description| {
        crate::log::error(&format!("GLFW ({err:?}): {description}"));
    });
    match glfw {
        Ok(glfw) => {
            GLFW.with(|g| *g.borrow_mut() = Some(glfw));
            true
        }
        Err(err) => {
            crate::log::error(&format!("GLFW initialization failed: {err:?}"));
            false
        }
    }
}

/// Sets a window hint applied by the next `create_window` call.
pub fn window_hint(hint: WindowHint) {
    GLFW.with(|g| {
        g.borrow_mut()
            .as_mut()
            .expect("GLFW not initialized")
            .window_hint(hint)
    });
}

/// Video mode of the primary monitor as `(width, height)`.
pub fn primary_video_mode() -> (c_int, c_int) {
    GLFW.with(|g| {
        let mut guard = g.borrow_mut();
        let glfw = guard.as_mut().expect("GLFW not initialized");
        glfw.with_primary_monitor(|_glfw, monitor| {
            monitor
                .and_then(|monitor| monitor.get_video_mode())
                .map(|mode| (mode.width as c_int, mode.height as c_int))
                .unwrap_or((1920, 1080))
        })
    })
}

/// Sets the monitor gamma. Raises GLFW_FEATURE_UNAVAILABLE on Wayland.
pub fn set_gamma(gamma: f32) {
    GLFW.with(|g| {
        let mut guard = g.borrow_mut();
        let glfw = guard.as_mut().expect("GLFW not initialized");
        glfw.with_primary_monitor(|_glfw, monitor| {
            if let Some(monitor) = monitor {
                monitor.set_gamma(gamma);
            }
        });
    });
}

/// Creates a windowed window (the sandbox maximizes it afterwards).
pub fn create_window(width: c_int, height: c_int, title: &str) -> Option<Window> {
    GLFW.with(|g| {
        let mut guard = g.borrow_mut();
        let glfw = guard.as_mut().expect("GLFW not initialized");
        glfw.create_window(width as u32, height as u32, title, WindowMode::Windowed)
            .map(|(window, _receiver)| Window { inner: window })
    })
}

pub fn poll_events() {
    GLFW.with(|g| {
        g.borrow_mut()
            .as_mut()
            .expect("GLFW not initialized")
            .poll_events()
    });
}

/// Shuts GLFW down. Must be called after the sandbox (and its window) is
/// dropped; dropping the last `Glfw` handle runs `glfwTerminate`.
pub fn terminate() {
    GLFW.with(|g| *g.borrow_mut() = None);
}

pub struct Window {
    inner: glfw::PWindow,
}

impl Window {
    /// The raw `GLFWwindow` handle, shared with the C++ core.
    pub fn raw(&self) -> *mut glfw::ffi::GLFWwindow {
        glfw::Context::window_ptr(&*self.inner)
    }

    pub fn raw_void(&self) -> *mut c_void {
        self.raw().cast()
    }

    pub fn should_close(&self) -> bool {
        self.inner.should_close()
    }

    pub fn maximize(&mut self) {
        self.inner.maximize();
    }

    pub fn framebuffer_size(&self) -> (c_int, c_int) {
        self.inner.get_framebuffer_size()
    }

    pub fn window_size(&self) -> (c_int, c_int) {
        self.inner.get_size()
    }

    pub fn content_scale(&self) -> (f32, f32) {
        self.inner.get_content_scale()
    }

    pub fn cursor_pos(&self) -> (f64, f64) {
        self.inner.get_cursor_pos()
    }

    pub fn set_framebuffer_size_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&mut glfw::Window, c_int, c_int) + 'static,
    {
        self.inner.set_framebuffer_size_callback(callback);
    }

    pub fn set_content_scale_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&mut glfw::Window, f32, f32) + 'static,
    {
        self.inner.set_content_scale_callback(callback);
    }

    pub fn set_mouse_button_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&mut glfw::Window, MouseButton, Action, Modifiers) + 'static,
    {
        self.inner.set_mouse_button_callback(callback);
    }

    pub fn set_cursor_pos_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&mut glfw::Window, f64, f64) + 'static,
    {
        self.inner.set_cursor_pos_callback(callback);
    }

    pub fn set_scroll_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&mut glfw::Window, f64, f64) + 'static,
    {
        self.inner.set_scroll_callback(callback);
    }

    pub fn set_key_callback<F>(&mut self, callback: F)
    where
        F: FnMut(&mut glfw::Window, Key, c_int, Action, Modifiers) + 'static,
    {
        self.inner.set_key_callback(callback);
    }
}
