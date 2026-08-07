//! FFI bindings to the C++ core (dev_sandbox/cshim/c_api.h).

use std::ffi::c_char;
use std::ffi::c_void;

// Log levels (base::LogLevel).
pub const LOG_INFO: i32 = 1;
pub const LOG_WARNING: i32 = 2;
pub const LOG_ERROR: i32 = 3;

// dp::ApiVersion.
pub const API_INVALID: i32 = -1;
pub const API_OPENGLES3: i32 = 0;
pub const API_METAL: i32 = 1;
pub const API_VULKAN: i32 = 2;

// df::TouchEvent::ETouchType.
pub const TOUCH_DOWN: i32 = 1;
pub const TOUCH_MOVE: i32 = 2;
pub const TOUCH_UP: i32 = 3;

// location::TLocationSource.
pub const LOCATION_SOURCE_USER: i32 = 7;

// location::EMyPositionMode.
pub const POSITION_PENDING: i32 = 0;
pub const POSITION_NOT_FOLLOW_NO_POS: i32 = 1;
pub const POSITION_NOT_FOLLOW: i32 = 2;
pub const POSITION_FOLLOW: i32 = 3;
pub const POSITION_FOLLOW_AND_ROTATE: i32 = 4;

// storage::Status (inner).
pub const STATUS_ON_DISK: i32 = 1;
pub const STATUS_NOT_DOWNLOADED: i32 = 2;
pub const STATUS_DOWNLOADING: i32 = 4;
pub const STATUS_IN_QUEUE: i32 = 6;
pub const STATUS_ON_DISK_OUT_OF_DATE: i32 = 8;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct OmPointD {
    pub x: f64,
    pub y: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct OmPointF {
    pub x: f32,
    pub y: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct OmTouch {
    pub location: OmPointF,
    pub id: i64,
    pub force: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct OmTouchEvent {
    pub type_: i32,
    pub first: OmTouch,
    pub second: OmTouch,
    pub has_second: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct OmGpsInfo {
    pub source: i32,
    pub timestamp: f64,
    pub latitude: f64,
    pub longitude: f64,
    pub horizontal_accuracy: f64,
    pub altitude: f64,
    pub vertical_accuracy: f64,
    pub bearing: f64,
    pub speed: f64,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct OmCompassInfo {
    pub bearing: f64,
}

// ImGui draw data mirroring dev_sandbox/cshim/c_api.h. Layout matches the
// imgui-rs types they are fed from (`DrawVert`, `ImDrawIdx`, `ImDrawCmd`).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct OmImGuiVertex {
    pub x: f32,
    pub y: f32,
    pub u: f32,
    pub v: f32,
    pub color: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct OmImGuiCmd {
    pub clip_x: f32,
    pub clip_y: f32,
    pub clip_z: f32,
    pub clip_w: f32,
    pub elem_count: u32,
    pub idx_offset: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct OmImGuiDrawList {
    pub vertices: *const OmImGuiVertex,
    pub vertex_count: u32,
    pub indices: *const u16,
    pub index_count: u32,
    pub cmds: *const OmImGuiCmd,
    pub cmd_count: u32,
}

pub type OmCountryChangedFn = unsafe extern "C" fn(user: *mut c_void, country_id: *const c_char);
pub type OmDownloadProgressFn =
    unsafe extern "C" fn(user: *mut c_void, country_id: *const c_char, downloaded: i64, total: i64);
pub type OmRenderInjectionFn = unsafe extern "C" fn(
    user: *mut c_void,
    context: *mut c_void,
    texture_manager: *mut c_void,
    program_manager: *mut c_void,
    shutdown: i32,
);

unsafe extern "C" {
    // Platform / logging.
    pub fn om_plat_cpu_cores() -> u32;
    pub fn om_plat_version(buf: *mut c_char, cap: usize);
    pub fn om_plat_setup_measurement();
    pub fn om_plat_set_gui_thread(task_loop: *mut c_void);
    pub fn om_settings_dev_mode_set(enabled: i32);
    pub fn om_settings_dev_mode_get() -> i32;
    pub fn om_log_message(level: i32, msg: *const c_char, len: usize);

    // GUI task loop.
    pub fn om_task_loop_new() -> *mut c_void;
    pub fn om_task_loop_execute(task_loop: *mut c_void);

    // Context factory.
    pub fn om_ctx_create(glfw_window: *mut c_void, api_version: i32, w: u32, h: u32)
    -> *mut c_void;
    pub fn om_ctx_delete(ctx_factory: *mut c_void);
    pub fn om_ctx_on_create_engine(
        glfw_window: *mut c_void,
        api_version: i32,
        ctx_factory: *mut c_void,
    );
    pub fn om_ctx_prepare_destroy(ctx_factory: *mut c_void);
    pub fn om_ctx_update_content_scale(glfw_window: *mut c_void, scale: f32);
    pub fn om_ctx_update_size(ctx_factory: *mut c_void, w: i32, h: i32);

    // Framework.
    pub fn om_fw_new(enable_diffs: i32) -> *mut c_void;
    pub fn om_fw_delete(f: *mut c_void);
    pub fn om_fw_set_callbacks(
        f: *mut c_void,
        user: *mut c_void,
        country_changed: Option<OmCountryChangedFn>,
        download_progress: Option<OmDownloadProgressFn>,
        render_injection: Option<OmRenderInjectionFn>,
    );
    pub fn om_fw_create_engine(
        f: *mut c_void,
        context_factory: *mut c_void,
        api_version: i32,
        visual_scale: f64,
        surface_width: i32,
        surface_height: i32,
    ) -> i32;
    pub fn om_fw_destroy_engine(f: *mut c_void);
    pub fn om_fw_set_render_enabled(f: *mut c_void);
    pub fn om_fw_set_render_disabled(f: *mut c_void, destroy_surface: i32);
    pub fn om_fw_api_version(f: *mut c_void) -> i32;
    pub fn om_fw_on_size(f: *mut c_void, w: i32, h: i32);
    pub fn om_fw_update_visual_scale(f: *mut c_void, vs: f64);
    pub fn om_fw_update_widgets(f: *mut c_void, w: i32, h: i32);
    pub fn om_fw_frame_active(f: *mut c_void);
    pub fn om_fw_enter_background(f: *mut c_void);
    pub fn om_fw_on_location(f: *mut c_void, info: *const OmGpsInfo);
    pub fn om_fw_on_compass(f: *mut c_void, info: *const OmCompassInfo);
    pub fn om_fw_next_position_mode(f: *mut c_void);
    pub fn om_fw_position_mode(f: *mut c_void) -> i32;
    pub fn om_fw_touch(f: *mut c_void, ev: *const OmTouchEvent);
    pub fn om_fw_scale(f: *mut c_void, factor: f64, px: f64, py: f64, animated: i32);
    pub fn om_fw_scale_zoom(f: *mut c_void, magnify: i32, animated: i32);
    pub fn om_fw_debug_rects(f: *mut c_void, enabled: i32);
    pub fn om_fw_set_posteffect_aa(f: *mut c_void, enabled: i32);
    pub fn om_fw_set_tile_background(f: *mut c_void, mode: i32, opacity: f32);
    pub fn om_fw_pto_g(f: *mut c_void, x: f64, y: f64) -> OmPointD;
    pub fn om_fw_pixel_center(f: *mut c_void) -> OmPointD;
    pub fn om_fw_country_id_valid(country_id: *const c_char) -> i32;
    pub fn om_fw_country_status(f: *mut c_void, country_id: *const c_char) -> i32;
    pub fn om_fw_country_size(f: *mut c_void, country_id: *const c_char) -> i64;
    pub fn om_fw_download_country(f: *mut c_void, country_id: *const c_char);
    pub fn om_fw_retry_download_country(f: *mut c_void, country_id: *const c_char);

    // ImGui -> drape backend (cshim/c_api.cpp).
    pub fn om_imgui_new() -> *mut c_void;
    pub fn om_imgui_delete(renderer: *mut c_void);
    pub fn om_imgui_set_texture(
        renderer: *mut c_void,
        width: u32,
        height: u32,
        rgba: *const u8,
        len: usize,
    );
    pub fn om_imgui_update(
        renderer: *mut c_void,
        lists: *const OmImGuiDrawList,
        list_count: u32,
        display_pos_x: f32,
        display_pos_y: f32,
        display_size_x: f32,
        display_size_y: f32,
        framebuffer_scale_x: f32,
        framebuffer_scale_y: f32,
    );
    pub fn om_imgui_render(
        renderer: *mut c_void,
        context: *mut c_void,
        texture_manager: *mut c_void,
        program_manager: *mut c_void,
    );
    pub fn om_imgui_reset(renderer: *mut c_void);
}
