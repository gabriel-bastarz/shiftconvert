use log::debug;
use tauri::{AppHandle, Manager};
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongPtrW, GetWindowRect, SetForegroundWindow, SetWindowLongPtrW, SetWindowPos,
    ShowWindow, GWL_EXSTYLE, GWL_STYLE, HWND_TOPMOST, SWP_FRAMECHANGED, SWP_NOACTIVATE,
    SWP_NOMOVE, SWP_NOSIZE, SWP_NOZORDER, SWP_SHOWWINDOW, SW_HIDE, SW_SHOW, SW_SHOWNOACTIVATE,
    WS_CAPTION, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_MAXIMIZEBOX, WS_MINIMIZEBOX, WS_SYSMENU,
    WS_THICKFRAME,
};

use crate::screen::{self, ScreenRect, WorkArea};

pub fn hwnd_of(app: &AppHandle) -> Result<HWND, String> {
    let window = app
        .get_webview_window("main")
        .ok_or_else(|| "janela principal não encontrada".to_string())?;
    window
        .hwnd()
        .map_err(|error| format!("não foi possível obter o HWND: {error}"))
}

fn set_no_activate(hwnd: HWND, enabled: bool) {
    unsafe {
        let current = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        let mut next = current | WS_EX_TOOLWINDOW.0 as isize;
        if enabled {
            next |= WS_EX_NOACTIVATE.0 as isize;
        } else {
            next &= !(WS_EX_NOACTIVATE.0 as isize);
        }
        if next != current {
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, next);
        }
    }
}

/// Remove o quadro (título e bordas) diretamente no estilo da janela.
///
/// O tao mantém `WS_CAPTION` no estilo de janelas de nível superior mesmo
/// quando `decorations: false`; sem esta limpeza o cliente fica ~16×39 px
/// menor que a janela e o cartão do prompt corta embaixo.
pub fn strip_window_frame(app: &AppHandle) -> Result<(), String> {
    let hwnd = hwnd_of(app)?;
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_STYLE) as u32;
        let frame = WS_CAPTION.0 | WS_THICKFRAME.0 | WS_SYSMENU.0
            | WS_MINIMIZEBOX.0
            | WS_MAXIMIZEBOX.0;
        let next = style & !frame;
        if next != style {
            SetWindowLongPtrW(hwnd, GWL_STYLE, next as isize);
            let _ = SetWindowPos(
                hwnd,
                None,
                0,
                0,
                0,
                0,
                SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
            debug!(
                "strip_window_frame: estilo 0x{:X} -> 0x{:X}",
                style, next
            );
        }
    }
    Ok(())
}

pub fn set_window_rect(
    app: &AppHandle,
    rect: ScreenRect,
    activate: bool,
) -> Result<(), String> {
    strip_window_frame(app)?;
    let hwnd = hwnd_of(app)?;
    set_no_activate(hwnd, !activate);

    unsafe {
        let flags = SWP_NOACTIVATE | SWP_SHOWWINDOW;
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            flags,
        )
        .map_err(|error| format!("não foi possível posicionar a janela: {error}"))?;

        if activate {
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = SetForegroundWindow(hwnd);
        } else {
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
    }

    Ok(())
}

/// Move/redimensiona sem mostrar e sem ativar. Usado antes da captura de tela.
pub fn position_window(app: &AppHandle, rect: ScreenRect) -> Result<(), String> {
    let hwnd = hwnd_of(app)?;
    set_no_activate(hwnd, true);
    unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            SWP_NOACTIVATE,
        )
        .map_err(|error| format!("não foi possível posicionar a janela: {error}"))?;
    }
    Ok(())
}

pub fn window_rect(app: &AppHandle) -> Result<ScreenRect, String> {
    let hwnd = hwnd_of(app)?;
    let mut rect = RECT::default();
    unsafe {
        GetWindowRect(hwnd, &mut rect)
            .map_err(|error| format!("não foi possível ler a posição da janela: {error}"))?;
    }
    Ok(ScreenRect::new(
        rect.left,
        rect.top,
        rect.right - rect.left,
        rect.bottom - rect.top,
    ))
}

pub fn show_no_activate(app: &AppHandle) -> Result<(), String> {
    let hwnd = hwnd_of(app)?;
    set_no_activate(hwnd, true);
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
    Ok(())
}

pub fn hide_window(app: &AppHandle) {
    if let Ok(hwnd) = hwnd_of(app) {
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}

/// Mostra o cartão inicial no centro do monitor atual.
pub fn show_prompt(app: &AppHandle) -> Result<(WorkArea, f64), String> {
    let (cursor_x, cursor_y) = screen::cursor_position();
    let area = screen::work_area_for_point(cursor_x, cursor_y);
    let scale = screen::dpi_scale_for_point(cursor_x, cursor_y);
    let width = (500.0 * scale).round() as i32;
    let height = (360.0 * scale).round() as i32;
    let rect = ScreenRect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );
    debug!(
        "show_prompt: área {}x{}@{},{} | escala {:.3} | janela {}x{}@{},{}",
        area.width,
        area.height,
        area.x,
        area.y,
        scale,
        rect.width,
        rect.height,
        rect.x,
        rect.y
    );
    set_window_rect(app, rect, true)?;
    Ok((area, scale))
}

/// Mostra o onboarding em uma janela ampla e centralizada. O tamanho é
/// limitado pela área útil do monitor para funcionar em 100%, 125%, 150% e
/// resoluções menores sem cortar o conteúdo.
pub fn show_onboarding(app: &AppHandle) -> Result<(WorkArea, f64), String> {
    let (cursor_x, cursor_y) = screen::cursor_position();
    let area = screen::work_area_for_point(cursor_x, cursor_y);
    let scale = screen::dpi_scale_for_point(cursor_x, cursor_y).max(0.5);

    let margin_css = 24.0;
    let max_width_css = (area.width as f64 / scale - margin_css * 2.0).max(320.0);
    let max_height_css = (area.height as f64 / scale - margin_css * 2.0).max(320.0);
    let width_css = 1120.0_f64.min(max_width_css);
    let height_css = 760.0_f64.min(max_height_css);
    let width = (width_css * scale).round() as i32;
    let height = (height_css * scale).round() as i32;
    let rect = ScreenRect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    );

    debug!(
        "show_onboarding: área {}x{}@{},{} | escala {:.3} | janela {}x{}@{},{}",
        area.width, area.height, area.x, area.y, scale, rect.width, rect.height, rect.x, rect.y
    );
    set_window_rect(app, rect, true)?;
    Ok((area, scale))
}
