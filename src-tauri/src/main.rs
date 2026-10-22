#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod conversion;
mod drag;
mod engines;
mod file_info;
mod overlay;
mod screen;

use std::path::PathBuf;

use log::{error, info};
use serde::Serialize;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{command, AppHandle, Emitter, Manager};

use conversion::ConversionRequest;
use drag::{HudInfo, PickerResponse};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupInfo {
    pub mode: String,
    pub work_x: i32,
    pub work_y: i32,
    pub work_width: i32,
    pub work_height: i32,
    pub scale: f64,
}

fn startup_info(app: &AppHandle) -> StartupInfo {
    let hwnd = overlay::hwnd_of(app).ok();
    let (area, scale) = match hwnd {
        Some(hwnd) => (
            screen::work_area_for_window(hwnd),
            screen::dpi_scale_for_window(hwnd),
        ),
        None => (
            screen::WorkArea {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            1.0,
        ),
    };
    StartupInfo {
        mode: "prompt".to_string(),
        work_x: area.x,
        work_y: area.y,
        work_width: area.width,
        work_height: area.height,
        scale,
    }
}

#[command]
fn ui_ready(app: AppHandle) -> Result<StartupInfo, String> {
    info!("Interface pronta; exibindo o cartão inicial.");
    if let Err(error) = overlay::show_prompt(&app) {
        error!("Falha ao exibir o cartão inicial: {}", error);
        return Err(error);
    }
    Ok(startup_info(&app))
}

#[command]
fn show_prompt_cmd(app: AppHandle) -> Result<StartupInfo, String> {
    overlay::show_prompt(&app)?;
    Ok(startup_info(&app))
}

#[command]
fn hide_overlay(app: AppHandle) {
    info!("hide_overlay chamado pelo frontend.");
    drag::force_hide(&app);
}

#[command]
fn start_conversion(app: AppHandle, request: ConversionRequest) -> Result<String, String> {
    conversion::start_conversion(app, request)
}

#[command]
fn cancel_conversion(conversion_id: String) -> bool {
    conversion::cancel_conversion(conversion_id)
}

#[command]
async fn pick_file(app: AppHandle) -> Result<PickerResponse, String> {
    drag::pick_file(app).await
}

#[command]
fn show_hud(app: AppHandle, anchor_x: f64, anchor_y: f64) -> Result<HudInfo, String> {
    drag::show_hud_at(&app, anchor_x, anchor_y)
}

#[command]
fn reveal_output(path: String) -> Result<(), String> {
    let path_buf = PathBuf::from(path.trim());
    if !path_buf.exists() {
        return Err("arquivo de saída não encontrado".to_string());
    }

    let arg = format!("/select,{}", path_buf.display());
    std::process::Command::new("explorer.exe")
        .arg(arg)
        .spawn()
        .map_err(|error| format!("não foi possível abrir o Explorer: {error}"))?;
    Ok(())
}

fn emit_show_prompt(app: &AppHandle) {
    match overlay::show_prompt(app) {
        Ok(_) => {
            let _ = app.emit("show-prompt", startup_info(app));
        }
        Err(error) => error!("Não foi possível mostrar o prompt: {}", error),
    }
}

fn build_tray(app: &tauri::App) -> tauri::Result<()> {
    let open_item = MenuItem::with_id(app, "open", "Abrir ShiftConvert", true, None::<&str>)?;
    let pick_item = MenuItem::with_id(app, "pick", "Escolher arquivo…", true, None::<&str>)?;
    let quit_item = MenuItem::with_id(app, "quit", "Sair", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open_item, &pick_item, &quit_item])?;

    let mut builder = TrayIconBuilder::new()
        .menu(&menu)
        .show_menu_on_left_click(false)
        .tooltip("ShiftConvert — Shift + arraste para converter")
        .on_menu_event(|app, event| match event.id.as_ref() {
            "open" => emit_show_prompt(app),
            "pick" => {
                emit_show_prompt(app);
                let _ = app.emit("tray-pick", ());
            }
            "quit" => {
                info!("Encerrando pelo menu da bandeja.");
                app.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                emit_show_prompt(tray.app_handle());
            }
        });

    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }

    builder.build(app)?;
    Ok(())
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    tauri::Builder::default()
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                if let Err(error) = window.set_decorations(false) {
                    error!("Não foi possível remover as decorações da janela: {}", error);
                }
            }
            if let Err(error) = drag::init(app.handle().clone()) {
                error!("Drop target OLE indisponível: {}", error);
            }
            build_tray(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                overlay::hide_window(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            ui_ready,
            show_prompt_cmd,
            hide_overlay,
            start_conversion,
            cancel_conversion,
            pick_file,
            show_hud,
            reveal_output
        ])
        .run(tauri::generate_context!())
        .expect("erro ao executar o ShiftConvert");
}
