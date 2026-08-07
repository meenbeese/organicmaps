//! All mutable sandbox state plus the GLFW/input event handlers.
//!
//! Everything runs on the main thread: GLFW callbacks fire inside
//! `glfw::poll_events()` and the C++->Rust callbacks fire inside
//! `om_task_loop_execute()` / framework calls, both from the main loop.
//! The sandbox therefore lives in a thread-local and is only ever aliased
//! briefly, never concurrently.

use std::cell::RefCell;
use std::ffi::CStr;
use std::ffi::CString;
use std::ffi::c_char;
use std::ffi::c_int;
use std::ffi::c_void;

use crate::c_api;
use crate::glfw;
use crate::mercator::y_to_lat;

thread_local! {
  static SANDBOX: RefCell<Option<Sandbox>> = const { RefCell::new(None) };
}

/// Stores the sandbox in the main-thread state and returns the opaque `user`
/// token handed to the C++ core. The token is never dereferenced: the C++
/// callbacks recover the sandbox from the thread-local state instead.
pub fn init(sandbox: Sandbox) -> *mut c_void {
    SANDBOX.with(|slot| *slot.borrow_mut() = Some(sandbox));
    std::ptr::NonNull::<c_void>::dangling().as_ptr()
}

/// Drops the sandbox (and with it the GLFW window).
pub fn shutdown() {
    SANDBOX.with(|slot| *slot.borrow_mut() = None);
}

/// Runs `f` with a mutable reference to the sandbox.
/// Only valid from the main thread; see the module docs.
pub fn with<R>(f: impl FnOnce(&mut Sandbox) -> R) -> R {
    SANDBOX.with(|slot| f(slot.borrow_mut().as_mut().expect("sandbox not initialized")))
}

pub struct Sandbox {
    pub framework: *mut c_void,
    pub context_factory: *mut c_void,
    pub task_loop: *mut c_void,
    pub window: glfw::Window,
    pub visual_scale: f32,
    pub fb_width: i32,
    pub fb_height: i32,

    // ImGui (drape backend renderer + Controls panel).
    pub imgui_renderer: *mut c_void,

    // Input state.
    pub touch_active: bool,
    pub touch_mods: c_int,
    pub last_lat_lon: Option<(f64, f64)>, // (lat, lon)
    pub bearing_enabled: bool,
    pub bearing: f64, // degrees from true North
    pub set_up_location_by_left_click: bool,

    // Map rendering settings.
    pub enable_debug_rect_rendering: bool,
    pub enable_aa: bool,
    pub current_tile_background: c_int,
    pub current_api: c_int, // index into the API combo labels

    // Download UI state (filled by the C++ -> Rust callbacks).
    pub download_button_label: Option<String>,
    pub retry_button_label: Option<String>,
    pub download_status_label: Option<String>,
    pub last_country: Option<String>,
}

impl Sandbox {
    pub fn new(
        framework: *mut c_void,
        task_loop: *mut c_void,
        window: glfw::Window,
        visual_scale: f32,
        fb_width: i32,
        fb_height: i32,
    ) -> Self {
        Self {
            framework,
            context_factory: std::ptr::null_mut(),
            task_loop,
            window,
            visual_scale,
            fb_width,
            fb_height,
            imgui_renderer: std::ptr::null_mut(),
            touch_active: false,
            touch_mods: 0,
            last_lat_lon: None,
            bearing_enabled: false,
            bearing: 0.0,
            set_up_location_by_left_click: false,
            enable_debug_rect_rendering: false,
            enable_aa: false,
            current_tile_background: 0,
            current_api: 0,
            download_button_label: None,
            retry_button_label: None,
            download_status_label: None,
            last_country: None,
        }
    }

    pub unsafe fn create_drape_engine(&mut self) {
        self.create_drape_engine_for(default_api());
    }

    pub unsafe fn create_drape_engine_for(&mut self, api: i32) {
        let ctx = c_api::om_ctx_create(
            self.window.raw_void(),
            api,
            self.fb_width as u32,
            self.fb_height as u32,
        );
        if ctx.is_null() {
            crate::log::error("failed to create graphics context factory");
            return;
        }
        self.context_factory = ctx;
        c_api::om_fw_create_engine(
            self.framework,
            ctx,
            api,
            self.visual_scale as f64,
            self.fb_width,
            self.fb_height,
        );
        c_api::om_ctx_on_create_engine(self.window.raw_void(), api, ctx);
        c_api::om_fw_set_render_enabled(self.framework);
    }

    pub unsafe fn destroy_drape_engine(&mut self) {
        if self.context_factory.is_null() {
            return;
        }
        c_api::om_fw_set_render_disabled(self.framework, 1);
        c_api::om_fw_destroy_engine(self.framework);
        c_api::om_ctx_prepare_destroy(self.context_factory);
        c_api::om_ctx_delete(self.context_factory);
        self.context_factory = std::ptr::null_mut();
    }

    fn on_resize(&mut self, w: i32, h: i32) {
        self.fb_width = w;
        self.fb_height = h;
        if w > 0 && h > 0 {
            unsafe {
                c_api::om_ctx_update_size(self.context_factory, w, h);
                c_api::om_fw_on_size(self.framework, w, h);
                c_api::om_fw_update_widgets(self.framework, w, h);
                c_api::om_fw_frame_active(self.framework);
            }
        }
    }

    fn on_content_scale(&mut self, xscale: f32, yscale: f32) {
        self.visual_scale = xscale.max(yscale);
        unsafe { c_api::om_fw_update_visual_scale(self.framework, self.visual_scale as f64) };

        // On macOS the window size is in points, the framework works in pixels.
        let (w, h) = self.scaled_window_size(xscale, yscale);

        if w != self.fb_width || h != self.fb_height {
            #[cfg(target_os = "macos")]
            unsafe {
                c_api::om_ctx_update_content_scale(self.window.raw_void(), xscale);
            }
            self.fb_width = w;
            self.fb_height = h;
            unsafe {
                c_api::om_ctx_update_size(self.context_factory, w, h);
                c_api::om_fw_on_size(self.framework, w, h);
            }
        }
    }

    /// Window size converted to pixels using the given content scale.
    fn scaled_window_size(&self, _xscale: f32, _yscale: f32) -> (i32, i32) {
        let (w, h) = self.window.window_size();
        #[cfg(target_os = "macos")]
        let (w, h) = ((w as f32 * _xscale) as i32, (h as f32 * _yscale) as i32);
        (w, h)
    }

    fn on_mouse_button(&mut self, x: f64, y: f64, button: c_int, action: c_int, mods: c_int) {
        crate::imgui::set_mouse_button(button, action == glfw::GLFW_PRESS);
        if crate::imgui::capture_mouse() {
            unsafe { c_api::om_fw_frame_active(self.framework) };
            return;
        }

        #[cfg(target_os = "macos")]
        let (x, y) = (x * self.visual_scale as f64, y * self.visual_scale as f64);

        let p = unsafe { c_api::om_fw_pto_g(self.framework, x, y) };
        self.last_lat_lon = Some((y_to_lat(p.y), p.x));

        if self.set_up_location_by_left_click {
            self.set_user_location();
            return;
        }

        if button == glfw::GLFW_MOUSE_BUTTON_LEFT && action == glfw::GLFW_PRESS {
            let ev = get_touch_event(self, x, y, mods, c_api::TOUCH_DOWN);
            unsafe { c_api::om_fw_touch(self.framework, &ev) };
            self.touch_active = true;
            self.touch_mods = mods;
        }

        if self.touch_active && action == glfw::GLFW_RELEASE {
            let ev = get_touch_event(self, x, y, 0, c_api::TOUCH_UP);
            unsafe { c_api::om_fw_touch(self.framework, &ev) };
            self.touch_active = false;
            self.touch_mods = 0;
        }
    }

    fn on_mouse_move(&mut self, x: f64, y: f64) {
        if crate::imgui::capture_mouse() {
            unsafe { c_api::om_fw_frame_active(self.framework) };
        }

        #[cfg(target_os = "macos")]
        let (x, y) = (x * self.visual_scale as f64, y * self.visual_scale as f64);

        if self.touch_active {
            let ev = get_touch_event(self, x, y, self.touch_mods, c_api::TOUCH_MOVE);
            unsafe { c_api::om_fw_touch(self.framework, &ev) };
        }
    }

    fn on_scroll(&mut self, x: f64, y: f64, _xoffset: f64, y_offset: f64) {
        crate::imgui::add_scroll(y_offset as f32);
        if crate::imgui::capture_mouse() {
            unsafe { c_api::om_fw_frame_active(self.framework) };
            return;
        }

        #[cfg(target_os = "macos")]
        let (x, y) = (x * self.visual_scale as f64, y * self.visual_scale as f64);

        const SENSITIVITY: f64 = 0.01;
        let factor = (y_offset * SENSITIVITY).exp();
        unsafe { c_api::om_fw_scale(self.framework, factor, x, y, 0) };
    }

    pub(crate) fn set_user_location(&mut self) {
        let Some((lat, lon)) = self.last_lat_lon else {
            return;
        };
        let gps = c_api::OmGpsInfo {
            source: c_api::LOCATION_SOURCE_USER,
            timestamp: unix_time_secs(),
            latitude: lat,
            longitude: lon,
            horizontal_accuracy: 10.0,
            altitude: 0.0,
            vertical_accuracy: 0.0,
            bearing: if self.bearing_enabled {
                self.bearing
            } else {
                -1.0
            },
            speed: 0.0,
        };
        unsafe { c_api::om_fw_on_location(self.framework, &gps) };
        if self.bearing_enabled {
            let compass = c_api::OmCompassInfo {
                bearing: self.bearing.to_radians(),
            };
            unsafe { c_api::om_fw_on_compass(self.framework, &compass) };
        }
    }

    fn update_country(&mut self, country_id: *const c_char) {
        self.download_button_label = None;
        self.retry_button_label = None;
        self.download_status_label = None;

        let name = country_id_str(country_id);
        self.last_country = Some(name.clone());
        if name.is_empty() {
            return;
        }
        if unsafe { c_api::om_fw_country_id_valid(cstr(&name).as_ptr()) } == 0 {
            return;
        }

        let status = unsafe { c_api::om_fw_country_status(self.framework, cstr(&name).as_ptr()) };
        match status {
            c_api::STATUS_NOT_DOWNLOADED => {
                let size =
                    unsafe { c_api::om_fw_country_size(self.framework, cstr(&name).as_ptr()) };
                let (units, amount) = format_map_size(size);
                self.download_button_label = Some(format!("Download ({name}) {amount}{units}"));
            }
            c_api::STATUS_IN_QUEUE => {
                self.download_status_label = Some(format!("{name} is waiting for downloading"));
            }
            s if s != c_api::STATUS_DOWNLOADING
                && s != c_api::STATUS_ON_DISK
                && s != c_api::STATUS_ON_DISK_OUT_OF_DATE =>
            {
                self.retry_button_label = Some(format!("Retry to download {name}"));
            }
            _ => {}
        }
    }
}

// ---------------------------------------------------------------------------
// C++ -> Rust callbacks (registered via om_fw_set_callbacks).
// ---------------------------------------------------------------------------

/// Called when the current country changes (both by the framework listener and
/// by the storage subscription; the shim already filters out non-leaf ids).
pub unsafe extern "C" fn on_country_changed(_user: *mut c_void, country_id: *const c_char) {
    with(|s| s.update_country(country_id));
}

/// Called during map downloading with the download progress.
pub unsafe extern "C" fn on_download_progress(
    _user: *mut c_void,
    country_id: *const c_char,
    downloaded: i64,
    total: i64,
) {
    with(|s| {
        if total > 0 {
            let name = country_id_str(country_id);
            s.download_status_label = Some(format!(
                "Downloading ({name}) {}%",
                downloaded * 100 / total
            ));
        }
        unsafe { c_api::om_fw_frame_active(s.framework) };
    });
}

// ---------------------------------------------------------------------------
// GLFW callbacks (registered on the window; fired inside glfw::poll_events).
// ---------------------------------------------------------------------------

/// Registers the window callbacks that feed the sandbox input handlers.
pub fn install_glfw_callbacks(window: &mut glfw::Window) {
    window.set_framebuffer_size_callback(|_window, width, height| {
        with(|s| s.on_resize(width, height));
    });

    window.set_content_scale_callback(|_window, xscale, yscale| {
        with(|s| s.on_content_scale(xscale, yscale));
    });

    window.set_mouse_button_callback(|window, button, action, mods| {
        let (x, y) = window.get_cursor_pos();
        with(|s| s.on_mouse_button(x, y, button as c_int, action as c_int, mods.bits()));
    });

    window.set_cursor_pos_callback(|_window, x, y| {
        with(|s| s.on_mouse_move(x, y));
    });

    window.set_scroll_callback(|window, x_offset, y_offset| {
        let (x, y) = window.get_cursor_pos();
        with(|s| s.on_scroll(x, y, x_offset, y_offset));
    });

    window.set_key_callback(|_window, _key, _scancode, _action, _mods| {});
}

fn default_api() -> i32 {
    #[cfg(target_os = "macos")]
    {
        c_api::API_METAL
    }
    #[cfg(not(target_os = "macos"))]
    {
        c_api::API_VULKAN
    }
}

/// Builds a TouchEvent. When the Super key is held, a second symmetrical
/// touch around the visible pixel center is added (two-finger gesture).
fn get_touch_event(s: &Sandbox, x: f64, y: f64, mods: c_int, type_: i32) -> c_api::OmTouchEvent {
    let first = c_api::OmTouch {
        location: c_api::OmPointF {
            x: x as f32,
            y: y as f32,
        },
        id: 0,
        force: 0.0,
    };
    let mut second = first;
    let mut has_second = 0;
    if mods & glfw::GLFW_MOD_SUPER != 0 {
        let center = unsafe { c_api::om_fw_pixel_center(s.framework) };
        second.location = symmetrical_touch(center, x, y);
        second.id = 1;
        has_second = 1;
    }
    c_api::OmTouchEvent {
        type_,
        first,
        second,
        has_second,
    }
}

/// The second touch of a Super-key gesture: the mirror image of `(x, y)`
/// across the visible pixel center.
fn symmetrical_touch(center: c_api::OmPointD, x: f64, y: f64) -> c_api::OmPointF {
    c_api::OmPointF {
        x: (2.0 * center.x - x) as f32,
        y: (2.0 * center.y - y) as f32,
    }
}

fn format_map_size(size: i64) -> (String, i64) {
    const MB: i64 = 1024 * 1024;
    const KB: i64 = 1024;
    if size > MB {
        ("MB".to_string(), (size + MB - 1) / MB)
    } else if size > KB {
        ("KB".to_string(), (size + KB - 1) / KB)
    } else {
        ("B".to_string(), size)
    }
}

fn country_id_str(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

fn cstr(value: &str) -> CString {
    CString::new(value).expect("string with NUL byte")
}

fn unix_time_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}
