use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{
    GetMonitorInfoW, MonitorFromPoint, MonitorFromWindow, HMONITOR, MONITOR_DEFAULTTONEAREST,
    MONITORINFO,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, GetDpiForWindow, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

#[derive(Debug, Clone, Copy, Default)]
pub struct WorkArea {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl WorkArea {
    pub fn right(self) -> i32 {
        self.x + self.width
    }

    pub fn bottom(self) -> i32 {
        self.y + self.height
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ScreenRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl ScreenRect {
    pub fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn clamp_to(self, area: WorkArea) -> Option<Self> {
        if self.width <= 0 || self.height <= 0 || area.width <= 0 || area.height <= 0 {
            return None;
        }

        let width = self.width.min(area.width);
        let height = self.height.min(area.height);
        let x = self.x.clamp(area.x, area.right() - width);
        let y = self.y.clamp(area.y, area.bottom() - height);
        Some(Self {
            x,
            y,
            width,
            height,
        })
    }
}

pub fn cursor_position() -> (i32, i32) {
    let mut point = POINT::default();
    unsafe {
        let _ = GetCursorPos(&mut point);
    }
    (point.x, point.y)
}

pub fn work_area_for_point(x: i32, y: i32) -> WorkArea {
    unsafe {
        let monitor = MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST);
        work_area_for_monitor(monitor).unwrap_or(WorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        })
    }
}

pub fn monitor_for_point(x: i32, y: i32) -> HMONITOR {
    unsafe { MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTONEAREST) }
}

pub fn dpi_scale_for_monitor(monitor: HMONITOR) -> f64 {
    unsafe {
        let mut dpi_x = 0u32;
        let mut dpi_y = 0u32;
        if GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y).is_ok()
            && dpi_x >= 48
        {
            dpi_x as f64 / 96.0
        } else {
            1.0
        }
    }
}

/// Escala do monitor que contém o ponto. É independente do DPI atual da
/// janela, que pode estar desatualizado enquanto ela ainda está oculta.
pub fn dpi_scale_for_point(x: i32, y: i32) -> f64 {
    dpi_scale_for_monitor(monitor_for_point(x, y))
}

pub fn work_area_for_window(hwnd: HWND) -> WorkArea {
    unsafe {
        let monitor = MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST);
        work_area_for_monitor(monitor).unwrap_or(WorkArea {
            x: 0,
            y: 0,
            width: 1920,
            height: 1080,
        })
    }
}

unsafe fn work_area_for_monitor(monitor: windows::Win32::Graphics::Gdi::HMONITOR) -> Option<WorkArea> {
    let mut info = MONITORINFO {
        cbSize: std::mem::size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    if !GetMonitorInfoW(monitor, &mut info).as_bool() {
        return None;
    }
    Some(WorkArea {
        x: info.rcWork.left,
        y: info.rcWork.top,
        width: info.rcWork.right - info.rcWork.left,
        height: info.rcWork.bottom - info.rcWork.top,
    })
}

pub fn dpi_scale_for_window(hwnd: HWND) -> f64 {
    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    if !monitor.0.is_null() {
        let scale = dpi_scale_for_monitor(monitor);
        if (scale - 1.0).abs() > f64::EPSILON {
            return scale;
        }
    }
    let dpi = unsafe { GetDpiForWindow(hwnd) };
    if dpi == 0 {
        1.0
    } else {
        dpi as f64 / 96.0
    }
}
