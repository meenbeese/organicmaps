use crate::c_api;
use crate::glfw;
use crate::imgui;
use crate::log;
use crate::sandbox;
use crate::sandbox::Sandbox;

/// Runs the sandbox. Returns Ok(()) on clean exit.
pub fn run() -> Result<(), String> {
    unsafe {
        c_api::om_plat_setup_measurement();
    }

    // Developer mode is enabled for the sandbox.
    if unsafe { c_api::om_settings_dev_mode_get() } == 0 {
        unsafe {
            c_api::om_settings_dev_mode_set(1);
        }
    }

    let version = platform_version();
    let cores = unsafe { c_api::om_plat_cpu_cores() };
    log::info(&format!(
        "Organic Maps: Developer Sandbox {} detected CPU cores: {cores}",
        version
    ));

    // GUI task loop: dispatch point for C++ code that wants the main thread.
    // Ownership moves to the Platform; we keep the raw pointer to drain it.
    let task_loop = unsafe { c_api::om_task_loop_new() };
    if task_loop.is_null() {
        return Err("failed to create task loop".to_string());
    }
    unsafe {
        c_api::om_plat_set_gui_thread(task_loop);
    }

    // Init GLFW (built from source by the glfw crate / glfw-sys).
    if !glfw::init() {
        return Err("GLFW initialization failed".to_string());
    }
    glfw::window_hint(glfw::WindowHint::ClientApi(glfw::ClientApiHint::NoApi));
    #[cfg(target_os = "windows")]
    glfw::window_hint(glfw::WindowHint::ScaleToMonitor(true));
    let (window_width, window_height) = glfw::primary_video_mode();
    let Some(mut window) = glfw::create_window(
        window_width,
        window_height,
        "Organic Maps: Developer Sandbox",
    ) else {
        glfw::terminate();
        return Err("GLFW window creation failed".to_string());
    };

    let (fb_width, fb_height) = window.framebuffer_size();
    let (xs, ys) = window.content_scale();
    let visual_scale = xs.max(ys);
    #[cfg(not(target_os = "linux"))]
    glfw::set_gamma(1.0);
    window.maximize();

    let framework = unsafe { c_api::om_fw_new(1) };
    if framework.is_null() {
        glfw::terminate();
        return Err("failed to create Framework".to_string());
    }
    log::info("Framework created");

    // Sandbox state is main-thread-local so that the GLFW and C++ callbacks
    // can reach it; init returns the opaque `user` token for the C++ core.
    let sandbox_ptr = sandbox::init(Sandbox::new(
        framework,
        task_loop,
        window,
        visual_scale,
        fb_width,
        fb_height,
    ));

    // ImGui context + font atlas + drape backend renderer.
    let imgui_renderer = unsafe { imgui::init() };
    if imgui_renderer.is_null() {
        return Err("failed to create ImGui renderer".to_string());
    }

    unsafe {
        c_api::om_fw_set_callbacks(
            framework,
            sandbox_ptr,
            Some(sandbox::on_country_changed),
            Some(sandbox::on_download_progress),
            Some(imgui::on_render_injection),
        );
    }

    sandbox::with(|s| {
        s.imgui_renderer = imgui_renderer;
        unsafe {
            s.create_drape_engine();
        }
        sandbox::install_glfw_callbacks(&mut s.window);
    });

    // Main loop.
    while !sandbox::with(|s| s.window.should_close()) {
        glfw::poll_events();

        // Drain tasks posted by C++ code to the GUI thread.
        unsafe {
            c_api::om_task_loop_execute(task_loop);
        }

        // Render the ImGui Controls panel.
        {
            let (vs, fbw, fbh, mx, my) = sandbox::with(|s| {
                let (mx, my) = s.window.cursor_pos();
                (s.visual_scale, s.fb_width, s.fb_height, mx, my)
            });
            unsafe {
                imgui::frame(vs, fbw, fbh, mx, my);
            }
        }

        std::thread::sleep(std::time::Duration::from_millis(1000 / 30));
    }

    // Teardown.
    sandbox::with(|s| {
        unsafe {
            c_api::om_fw_enter_background(s.framework);
        }
        unsafe {
            s.destroy_drape_engine();
        }
    });
    unsafe {
        imgui::shutdown();
    }
    // Dropping the sandbox destroys the GLFW window; terminate GLFW after.
    sandbox::shutdown();
    glfw::terminate();

    unsafe {
        c_api::om_fw_delete(framework);
    }
    log::info("Shutdown complete");
    Ok(())
}

fn platform_version() -> String {
    let mut buf = [0i8; 256];
    unsafe {
        c_api::om_plat_version(buf.as_mut_ptr(), buf.len());
    }
    let cstr = unsafe { std::ffi::CStr::from_ptr(buf.as_ptr()) };
    cstr.to_string_lossy().into_owned()
}
