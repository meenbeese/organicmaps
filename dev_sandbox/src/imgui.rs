//! imgui-rs UI integration.
//!
//! The Rust shell owns the `imgui::Context` (main thread only), forwards GLFW
//! input to it, and ships each frame's draw data plus the font atlas texture
//! into the C++ drape backend renderer (defined in dev_sandbox/cshim/c_api.cpp),
//! which uploads and draws it on the render thread.

// The ImGui context and input state globals are only ever touched from the
// main thread (GLFW callbacks, task-loop drain, and the main loop), so the
// references below never race.

use std::cell::RefCell;
use std::ffi::c_void;
use std::sync::atomic::AtomicPtr;
use std::sync::atomic::Ordering;

use imgui::ConfigFlags;
use imgui::DrawCmd;

use crate::c_api;
use crate::ui;

/// The drape backend renderer. Written once from the main thread before the
/// render thread starts; read afterwards by the render-injection callback
/// (which runs on drape's render thread).
static RENDERER: AtomicPtr<c_void> = AtomicPtr::new(std::ptr::null_mut());

const MOUSE_BUTTONS: usize = 5;

/// Per-frame ImGui state. Main thread only; see the module docs.
#[derive(Default)]
struct UiState {
    ctx: Option<imgui::Context>,
    mouse_down: [bool; MOUSE_BUTTONS],
    scroll: f32,
    last_frame_secs: f64,
}

thread_local! {
  static UI_STATE: RefCell<UiState> = RefCell::new(UiState::default());
}

/// Creates the ImGui context, builds and uploads the font atlas, and creates
/// the drape backend renderer. Returns the renderer handle.
pub unsafe fn init() -> *mut c_void {
    let mut ctx = imgui::Context::create();
    ctx.set_ini_filename(None);
    ctx.io_mut().config_flags |= ConfigFlags::NAV_ENABLE_KEYBOARD;

    // Build the font atlas once and copy it into the drape renderer. The
    // borrow ends before `ctx` is moved into the global.
    let (width, height, data) = {
        let fonts = ctx.fonts();
        let atlas = fonts.build_rgba32_texture();
        (atlas.width, atlas.height, atlas.data.to_vec())
    };
    let renderer = c_api::om_imgui_new();
    c_api::om_imgui_set_texture(renderer, width, height, data.as_ptr(), data.len());

    UI_STATE.with(|st| st.borrow_mut().ctx = Some(ctx));
    RENDERER.store(renderer, Ordering::Relaxed);
    renderer
}

/// Tears the UI down. Must run on the main thread after the render thread
/// stopped.
pub unsafe fn shutdown() {
    let renderer = RENDERER.swap(std::ptr::null_mut(), Ordering::Relaxed);
    if !renderer.is_null() {
        unsafe { c_api::om_imgui_delete(renderer) };
    }
    UI_STATE.with(|st| st.borrow_mut().ctx = None);
}

/// Render-injection callback invoked by drape on the render thread.
pub unsafe extern "C" fn on_render_injection(
    _user: *mut c_void,
    context: *mut c_void,
    texture_manager: *mut c_void,
    program_manager: *mut c_void,
    shutdown: i32,
) {
    let renderer = RENDERER.load(Ordering::Relaxed);
    if renderer.is_null() {
        return;
    }
    if shutdown != 0 {
        unsafe { c_api::om_imgui_reset(renderer) };
    } else {
        unsafe { c_api::om_imgui_render(renderer, context, texture_manager, program_manager) };
    }
}

/// Whether ImGui wants to receive mouse input (hover or drag over a window).
/// Reflects the previous frame; GLFW input events use it to decide whether to
/// route an event to ImGui or to the map.
pub fn capture_mouse() -> bool {
    UI_STATE.with(|st| {
        st.borrow()
            .ctx
            .as_ref()
            .is_some_and(|ctx| ctx.io().want_capture_mouse)
    })
}

/// Records a mouse button press/release from a GLFW callback.
pub fn set_mouse_button(button: i32, pressed: bool) {
    if button >= 0 && (button as usize) < MOUSE_BUTTONS {
        UI_STATE.with(|st| st.borrow_mut().mouse_down[button as usize] = pressed);
    }
}

/// Accumulates a scroll delta from a GLFW callback.
pub fn add_scroll(delta: f32) {
    UI_STATE.with(|st| st.borrow_mut().scroll += delta);
}

/// Produces one frame of UI and ships the draw data to the drape renderer.
/// `cursor` is the GLFW cursor position in window coordinates.
pub unsafe fn frame(
    visual_scale: f32,
    fb_width: i32,
    fb_height: i32,
    cursor_x: f64,
    cursor_y: f64,
) {
    let renderer = RENDERER.load(Ordering::Relaxed);
    if renderer.is_null() {
        return;
    }

    UI_STATE.with(|st| {
        let mut st = st.borrow_mut();

        let now = unix_time_secs();
        let delta = if st.last_frame_secs > 0.0 {
            now - st.last_frame_secs
        } else {
            1.0 / 60.0
        };
        st.last_frame_secs = now;
        let mouse_down = st.mouse_down;
        let scroll = st.scroll;

        let ctx = match st.ctx.as_mut() {
            Some(ctx) => ctx,
            None => return,
        };

        let io = ctx.io_mut();
        io.display_size = [
            fb_width as f32 / visual_scale,
            fb_height as f32 / visual_scale,
        ];
        io.display_framebuffer_scale = [visual_scale, visual_scale];
        io.delta_time = delta.max(1e-4) as f32;
        io.mouse_pos = mouse_to_display(cursor_x, cursor_y, visual_scale);
        io.mouse_down = mouse_down;
        io.mouse_wheel = scroll;

        let ui = ctx.frame();
        crate::sandbox::with(|s| ui::update(ui, s));

        let draw_data = ctx.render();

        // The first frame after context creation may produce no draw lists.
        if draw_data.draw_lists_count() == 0 {
            return;
        }

        // Convert the imgui-rs draw data to the C ABI mirror. `DrawVert` and
        // `OmImGuiVertex` have identical layouts, so the vertex/index slices are
        // passed by pointer without copying; the cmd structs are owned by Rust
        // buffers that must outlive the FFI call.
        let mut cmd_buffers: Vec<Vec<c_api::OmImGuiCmd>> =
            Vec::with_capacity(draw_data.draw_lists_count());
        let mut lists: Vec<c_api::OmImGuiDrawList> =
            Vec::with_capacity(draw_data.draw_lists_count());
        for draw_list in draw_data.draw_lists() {
            let mut cmds = Vec::new();
            for cmd in draw_list.commands() {
                if let DrawCmd::Elements { count, cmd_params } = cmd {
                    let clip = cmd_params.clip_rect;
                    cmds.push(c_api::OmImGuiCmd {
                        clip_x: clip[0],
                        clip_y: clip[1],
                        clip_z: clip[2],
                        clip_w: clip[3],
                        elem_count: count as u32,
                        idx_offset: cmd_params.idx_offset as u32,
                    });
                }
            }
            let verts = draw_list.vtx_buffer();
            let idx = draw_list.idx_buffer();
            lists.push(c_api::OmImGuiDrawList {
                vertices: verts.as_ptr().cast(),
                vertex_count: verts.len() as u32,
                indices: idx.as_ptr(),
                index_count: idx.len() as u32,
                cmds: cmds.as_ptr(),
                cmd_count: cmds.len() as u32,
            });
            cmd_buffers.push(cmds);
        }

        c_api::om_imgui_update(
            renderer,
            lists.as_ptr(),
            lists.len() as u32,
            draw_data.display_pos[0],
            draw_data.display_pos[1],
            draw_data.display_size[0],
            draw_data.display_size[1],
            draw_data.framebuffer_scale[0],
            draw_data.framebuffer_scale[1],
        );
    });
}

/// Converts the GLFW cursor position (window coordinates) to ImGui display
/// coordinates. On Linux window and framebuffer pixels coincide even for a
/// non-1.0 visual scale, so divide by the scale; on other platforms GLFW
/// cursor coordinates already match the display coordinate system.
#[cfg(target_os = "linux")]
fn mouse_to_display(x: f64, y: f64, visual_scale: f32) -> [f32; 2] {
    [x as f32 / visual_scale, y as f32 / visual_scale]
}

#[cfg(not(target_os = "linux"))]
fn mouse_to_display(x: f64, y: f64, _visual_scale: f32) -> [f32; 2] {
    [x as f32, y as f32]
}

fn unix_time_secs() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64())
        .unwrap_or(0.0)
}
