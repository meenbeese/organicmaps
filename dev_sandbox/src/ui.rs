//! ImGui Controls panel. Runs on the main thread every frame.

use imgui::Condition;
use imgui::Ui;

use crate::c_api;
use crate::sandbox::Sandbox;

/// Renders the Controls panel. Called from `imgui::frame` each frame.
pub unsafe fn update(ui: &Ui, s: &mut Sandbox) {
    ui.window("Controls")
        .position([5.0, 20.0], Condition::Appearing)
        .always_auto_resize(true)
        .build(|| {
            // Drape controls.
            let api_labels = api_labels();
            let mut api_index = s.current_api as usize;
            if ui.combo_simple_string("API", &mut api_index, api_labels) {
                let api_version = api_version_from_label(api_labels[api_index]);
                if unsafe { c_api::om_fw_api_version(s.framework) } != api_version {
                    unsafe {
                        s.destroy_drape_engine();
                        s.create_drape_engine_for(api_version);
                        c_api::om_fw_debug_rects(
                            s.framework,
                            i32::from(s.enable_debug_rect_rendering),
                        );
                        c_api::om_fw_set_posteffect_aa(s.framework, i32::from(s.enable_aa));
                        c_api::om_fw_set_tile_background(
                            s.framework,
                            s.current_tile_background,
                            0.5,
                        );
                    }
                }
            }
            if ui.checkbox("Debug rect rendering", &mut s.enable_debug_rect_rendering) {
                unsafe {
                    c_api::om_fw_debug_rects(s.framework, i32::from(s.enable_debug_rect_rendering))
                };
            }
            if ui.checkbox("Antialiasing", &mut s.enable_aa) {
                unsafe { c_api::om_fw_set_posteffect_aa(s.framework, i32::from(s.enable_aa)) };
            }
            ui.new_line();
            ui.separator();
            ui.new_line();

            // Map controls.
            if ui.button("Scale +") {
                unsafe { c_api::om_fw_scale_zoom(s.framework, 1, 1) };
            }
            ui.same_line();
            if ui.button("Scale -") {
                unsafe { c_api::om_fw_scale_zoom(s.framework, 0, 1) };
            }
            ui.checkbox(
                "Set up location by left click",
                &mut s.set_up_location_by_left_click,
            );
            if s.set_up_location_by_left_click {
                if ui.checkbox("Bearing", &mut s.bearing_enabled) {
                    s.set_user_location();
                }
                ui.same_line();
                let mut bearing = s.bearing as f32;
                if ui
                    .slider_config(" ", 0.0f32, 360.0f32)
                    .display_format("%.1f")
                    .build(&mut bearing)
                {
                    s.bearing = bearing as f64;
                    s.set_user_location();
                }
            }
            let mode = unsafe { c_api::om_fw_position_mode(s.framework) };
            ui.text(&format!("My positon mode: {}", position_text(mode)));
            if ui.button("Next Position Mode") {
                unsafe { c_api::om_fw_next_position_mode(s.framework) };
            }
            ui.new_line();
            ui.separator();
            ui.new_line();

            // No downloading on Linux at the moment (needs http_thread without Qt).
            #[cfg(not(target_os = "linux"))]
            download_controls(ui, s);

            let tile_background_labels = ["Default", "Satellite"];
            let mut tile_background_index = s.current_tile_background as usize;
            if ui.combo_simple_string(
                "Tile Background",
                &mut tile_background_index,
                &tile_background_labels,
            ) {
                s.current_tile_background = tile_background_index as i32;
                unsafe {
                    c_api::om_fw_set_tile_background(s.framework, s.current_tile_background, 0.5)
                };
            }
            ui.new_line();
            ui.separator();
            ui.new_line();
        });
}

#[cfg(not(target_os = "linux"))]
fn download_controls(ui: &Ui, s: &mut Sandbox) {
    if let Some(label) = s.download_button_label.clone() {
        if ui.button(&label) {
            if let Some(country) = s.last_country.clone() {
                let country = CString::new(country).expect("country id with NUL byte");
                unsafe { c_api::om_fw_download_country(s.framework, country.as_ptr()) };
            }
        }
    }
    if let Some(label) = s.retry_button_label.clone() {
        if ui.button(&label) {
            if let Some(country) = s.last_country.clone() {
                let country = CString::new(country).expect("country id with NUL byte");
                unsafe { c_api::om_fw_retry_download_country(s.framework, country.as_ptr()) };
            }
        }
    }
    if let Some(label) = s.download_status_label.clone() {
        ui.text(&label);
    }
    if s.download_button_label.is_some()
        || s.retry_button_label.is_some()
        || s.download_status_label.is_some()
    {
        ui.new_line();
        ui.separator();
        ui.new_line();
    }
}

#[cfg(not(target_os = "linux"))]
use std::ffi::CString;

fn api_labels() -> &'static [&'static str] {
    #[cfg(target_os = "macos")]
    {
        &["Metal", "Vulkan", "OpenGL"]
    }
    #[cfg(target_os = "linux")]
    {
        &["Vulkan", "OpenGL"]
    }
    #[cfg(target_os = "windows")]
    {
        &["Vulkan"]
    }
}

fn api_version_from_label(label: &str) -> i32 {
    match label {
        "Metal" => c_api::API_METAL,
        "Vulkan" => c_api::API_VULKAN,
        "OpenGL" => c_api::API_OPENGLES3,
        _ => c_api::API_INVALID,
    }
}

fn position_text(mode: i32) -> &'static str {
    match mode {
        c_api::POSITION_PENDING => "Pending",
        c_api::POSITION_NOT_FOLLOW_NO_POS => "No position",
        c_api::POSITION_NOT_FOLLOW => "Not follow",
        c_api::POSITION_FOLLOW => "Follow",
        c_api::POSITION_FOLLOW_AND_ROTATE => "Follow and Rotate",
        _ => "",
    }
}
