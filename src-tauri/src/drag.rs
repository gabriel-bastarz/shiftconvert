use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicIsize, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::Duration;

use log::{debug, error, info, warn};
use serde::Serialize;
use tauri::{AppHandle, Emitter};
use windows::core::{implement, w, Ref};
use windows::Win32::Foundation::{
    COLORREF, HINSTANCE, HWND, LPARAM, LRESULT, POINT, POINTL, RPC_E_CHANGED_MODE, WPARAM,
};
use windows::Win32::Graphics::Gdi::{BeginPaint, EndPaint, PAINTSTRUCT};
use windows::Win32::System::Com::{
    IDataObject, DVASPECT_CONTENT, FORMATETC, TYMED_HGLOBAL,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::System::Ole::{
    IDropTarget, IDropTarget_Impl, OleInitialize, RegisterDragDrop, ReleaseStgMedium, CF_HDROP,
    DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE,
};
use windows::Win32::System::SystemServices::MODIFIERKEYS_FLAGS;
use windows::Win32::UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_ESCAPE, VK_LBUTTON, VK_SHIFT};
use windows::Win32::UI::Shell::{DragQueryFileW, HDROP};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, GetAncestor, GetClassNameW, GetCursorPos,
    GetForegroundWindow, GetParent, RegisterClassW, SetLayeredWindowAttributes, SetWindowPos,
    ShowWindow, WindowFromPoint, CS_HREDRAW, CS_VREDRAW, GA_ROOT, HTCLIENT, HWND_TOPMOST,
    LWA_ALPHA, MA_NOACTIVATE, SW_HIDE, SW_SHOWNOACTIVATE, SWP_NOACTIVATE, SWP_SHOWWINDOW,
    WM_ERASEBKGND, WM_MOUSEACTIVATE, WM_NCHITTEST, WM_PAINT, WM_SETCURSOR, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
};

use crate::conversion::{self, ConversionRequest};
use crate::file_info::{self, FileInfo, TargetFormat};
use crate::overlay;
use crate::screen::{self, ScreenRect, WorkArea};

const TAU: f64 = std::f64::consts::TAU;

// ---------------------------------------------------------------------------
// Payloads enviados ao frontend
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OverlayContext {
    pub window_x: i32,
    pub window_y: i32,
    pub window_width: i32,
    pub window_height: i32,
    pub work_x: i32,
    pub work_y: i32,
    pub work_width: i32,
    pub work_height: i32,
    pub scale: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WheelLayout {
    pub center_x: f64,
    pub center_y: f64,
    pub outer_radius: f64,
    pub inner_radius: f64,
    pub rotation: f64,
    pub seam_bends: Vec<f64>,
    pub outer_wave: Vec<f64>,
    pub outer_phase: Vec<f64>,
    pub inner_wave: Vec<f64>,
    pub inner_phase: Vec<f64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Backdrop {
    pub data_url: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WheelPayload {
    pub mode: String,
    pub file: FileInfo,
    pub targets: Vec<TargetFormat>,
    pub layout: WheelLayout,
    pub backdrop: Option<Backdrop>,
    pub overlay: OverlayContext,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DragOverPayload {
    pub cursor_x: f64,
    pub cursor_y: f64,
    pub region_index: Option<usize>,
    pub target_id: Option<String>,
    pub angle: f64,
    pub distance: f64,
    pub over_center: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DropPayload {
    pub file: FileInfo,
    pub target: TargetFormat,
    pub conversion_id: String,
    pub layout: WheelLayout,
    pub overlay: OverlayContext,
    pub cursor_x: f64,
    pub cursor_y: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DragMessage {
    pub message: String,
    pub cursor_x: f64,
    pub cursor_y: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PickerResponse {
    pub cancelled: bool,
    pub wheel: Option<WheelPayload>,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HudInfo {
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OnboardingFileInfo {
    pub file: FileInfo,
    pub targets: Vec<TargetFormat>,
}

// ---------------------------------------------------------------------------
// Estado compartilhado
// ---------------------------------------------------------------------------

struct DragSession {
    file: FileInfo,
    targets: Vec<TargetFormat>,
    layout: WheelLayout,
    overlay: OverlayContext,
}

struct RuntimeState {
    session: Option<DragSession>,
    window_rect: ScreenRect,
    work_area: WorkArea,
    scale: f64,
}

impl Default for RuntimeState {
    fn default() -> Self {
        Self {
            session: None,
            window_rect: ScreenRect::new(0, 0, 0, 0),
            work_area: WorkArea {
                x: 0,
                y: 0,
                width: 1920,
                height: 1080,
            },
            scale: 1.0,
        }
    }
}

struct DropContext {
    app: AppHandle,
    state: Mutex<RuntimeState>,
    cancelled: AtomicBool,
    dropped: AtomicBool,
    overlay_hwnd: AtomicIsize,
    shield_hwnd: AtomicIsize,
}

impl DropContext {
    fn new(app: AppHandle) -> Self {
        Self {
            app,
            state: Mutex::new(RuntimeState::default()),
            cancelled: AtomicBool::new(false),
            dropped: AtomicBool::new(false),
            overlay_hwnd: AtomicIsize::new(0),
            shield_hwnd: AtomicIsize::new(0),
        }
    }

    fn lock(&self) -> MutexGuard<'_, RuntimeState> {
        self.state.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    fn emit<T: Serialize + Clone>(&self, event: &str, payload: T) {
        if let Err(error) = self.app.emit(event, payload) {
            warn!("Falha ao emitir {}: {}", event, error);
        }
    }
}

static DROP_CONTEXT: OnceLock<Arc<DropContext>> = OnceLock::new();
static ACTIVE: AtomicBool = AtomicBool::new(false);
static ONBOARDING_ACTIVE: AtomicBool = AtomicBool::new(false);

/// Enquanto o onboarding está aberto, o gesto global de Shift + arrastar fica
/// suspenso. A janela do onboarding é a mesma janela do overlay; sem esta
/// trava, um arraste iniciado no Explorer poderia abrir a roda por cima do
/// guia e capturar os eventos de teclado.
pub fn set_onboarding_active(active: bool) {
    ONBOARDING_ACTIVE.store(active, Ordering::SeqCst);
    if active {
        hide_shield();
        ACTIVE.store(false, Ordering::SeqCst);
        if let Some(context) = DROP_CONTEXT.get() {
            context.cancelled.store(false, Ordering::SeqCst);
            context.dropped.store(false, Ordering::SeqCst);
            context.lock().session = None;
        }
    }
}

// ---------------------------------------------------------------------------
// Inicialização COM + janela de captura OLE
// ---------------------------------------------------------------------------

pub fn init(app: AppHandle) -> Result<(), String> {
    unsafe {
        if let Err(error) = OleInitialize(None) {
            if error.code() != RPC_E_CHANGED_MODE {
                return Err(format!("OleInitialize falhou: {error}"));
            }
            warn!("OLE já inicializado em outro apartment; continuando.");
        }
    }

    let context = Arc::new(DropContext::new(app.clone()));
    if let Ok(hwnd) = overlay::hwnd_of(&app) {
        context.overlay_hwnd.store(hwnd.0 as isize, Ordering::SeqCst);
    }

    let shield = create_shield_window()?;
    let target: IDropTarget = ShiftDropTarget {
        context: context.clone(),
    }
    .into();

    unsafe {
        RegisterDragDrop(shield, &target)
            .map_err(|error| format!("RegisterDragDrop falhou: {error}"))?;
        let _ = SetLayeredWindowAttributes(shield, COLORREF(0), 1, LWA_ALPHA);
    }

    context.shield_hwnd.store(shield.0 as isize, Ordering::SeqCst);
    // RegisterDragDrop faz AddRef; a referência COM permanece viva mesmo
    // depois que este wrapper local sai de escopo.
    let _ = DROP_CONTEXT.set(context.clone());

    info!("Drop target OLE registrado na janela de captura {:?}", shield.0);
    spawn_watcher(context);
    Ok(())
}

fn create_shield_window() -> Result<HWND, String> {
    unsafe {
        let module = GetModuleHandleW(None)
            .map_err(|error| format!("GetModuleHandleW falhou: {error}"))?;
        let instance = HINSTANCE(module.0);

        let class = WNDCLASSW {
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(shield_wndproc),
            hInstance: instance,
            lpszClassName: w!("ShiftConvertDropShield"),
            ..Default::default()
        };

        let atom = RegisterClassW(&class);
        if atom == 0 {
            debug!("RegisterClassW retornou 0 (classe já registrada?)");
        }

        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_LAYERED,
            w!("ShiftConvertDropShield"),
            w!("ShiftConvertDropShield"),
            WS_POPUP,
            0,
            0,
            1,
            1,
            None,
            None,
            Some(instance),
            None,
        )
        .map_err(|error| format!("CreateWindowExW falhou: {error}"))?;

        let _ = SetLayeredWindowAttributes(hwnd, COLORREF(0), 1, LWA_ALPHA);
        Ok(hwnd)
    }
}

unsafe extern "system" fn shield_wndproc(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match message {
        WM_NCHITTEST => LRESULT(HTCLIENT as isize),
        WM_MOUSEACTIVATE => LRESULT(MA_NOACTIVATE as isize),
        WM_ERASEBKGND => LRESULT(1),
        WM_SETCURSOR => LRESULT(1),
        WM_PAINT => {
            let mut paint = PAINTSTRUCT::default();
            let _ = BeginPaint(hwnd, &mut paint);
            let _ = EndPaint(hwnd, &paint);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, message, wparam, lparam),
    }
}

// ---------------------------------------------------------------------------
// IDropTarget
// ---------------------------------------------------------------------------

#[implement(IDropTarget)]
struct ShiftDropTarget {
    context: Arc<DropContext>,
}

impl IDropTarget_Impl for ShiftDropTarget_Impl {
    fn DragEnter(
        &self,
        pdataobj: Ref<IDataObject>,
        _grfkeystate: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        unsafe {
            *pdweffect = DROPEFFECT_NONE;
        }

        let Some(data_object) = pdataobj.as_ref() else {
            return Ok(());
        };

        let (count, path) = match unsafe { extract_hdrop(data_object) } {
            Some(value) => value,
            None => {
                emit_invalid(&self.context, "Não foi possível ler o arquivo arrastado.", pt);
                return Ok(());
            }
        };

        if count > 1 {
            emit_invalid(&self.context, "Arraste apenas um arquivo por vez.", pt);
            return Ok(());
        }

        let file = match file_info::inspect_file(&path) {
            Ok(file) => file,
            Err(message) => {
                emit_invalid(&self.context, &message, pt);
                return Ok(());
            }
        };

        if !file.is_readable {
            emit_invalid(&self.context, "Sem permissão para ler este arquivo.", pt);
            return Ok(());
        }

        let targets = file_info::supported_targets(std::path::Path::new(&file.path));
        if targets.is_empty() {
            emit_invalid(
                &self.context,
                "Nenhuma conversão real disponível para este arquivo.",
                pt,
            );
            return Ok(());
        }

        let (window_rect, work_area, scale) = {
            let state = self.context.lock();
            (state.window_rect, state.work_area, state.scale)
        };

        let cursor_x = (pt.x as f64 - window_rect.x as f64) / scale.max(0.1);
        let cursor_y = (pt.y as f64 - window_rect.y as f64) / scale.max(0.1);
        let window_width = window_rect.width as f64 / scale.max(0.1);
        let window_height = window_rect.height as f64 / scale.max(0.1);

        let layout = make_layout(
            cursor_x,
            cursor_y,
            window_width,
            window_height,
            targets.len(),
            scale,
        );
        let overlay = OverlayContext {
            window_x: window_rect.x,
            window_y: window_rect.y,
            window_width: window_rect.width,
            window_height: window_rect.height,
            work_x: work_area.x,
            work_y: work_area.y,
            work_width: work_area.width,
            work_height: work_area.height,
            scale,
        };

        // A roda Orbit Minimal é opaca; não há mais captura do desktop nem
        // refração para alimentar o SVG.
        let backdrop: Option<Backdrop> = None;

        let payload = WheelPayload {
            mode: "drag".to_string(),
            file: file.clone(),
            targets: targets.clone(),
            layout: layout.clone(),
            backdrop,
            overlay: overlay.clone(),
        };

        self.context.lock().session = Some(DragSession {
            file,
            targets,
            layout,
            overlay,
        });

        debug!(
            "DragEnter: {} ({} formatos), cursor=({:.0},{:.0})",
            payload.file.file_name,
            payload.targets.len(),
            cursor_x,
            cursor_y
        );
        debug!(
            "Geometria: janela {}x{}@{},{} | área {}x{}@{},{} | escala {:.3} | centro ({:.1},{:.1}) raio externo {:.1} interno {:.1}",
            window_rect.width,
            window_rect.height,
            window_rect.x,
            window_rect.y,
            work_area.width,
            work_area.height,
            work_area.x,
            work_area.y,
            scale,
            payload.layout.center_x,
            payload.layout.center_y,
            payload.layout.outer_radius,
            payload.layout.inner_radius
        );
        self.context.emit("drag-enter", payload);

        unsafe {
            *pdweffect = DROPEFFECT_COPY;
        }
        Ok(())
    }

    fn DragOver(
        &self,
        _grfkeystate: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        unsafe {
            *pdweffect = DROPEFFECT_NONE;
        }

        if self.context.cancelled.load(Ordering::SeqCst) {
            return Ok(());
        }

        let (layout, overlay) = {
            let state = self.context.lock();
            match state.session.as_ref() {
                Some(session) => (session.layout.clone(), session.overlay.clone()),
                None => return Ok(()),
            }
        };

        let cursor_x = (pt.x as f64 - overlay.window_x as f64) / overlay.scale.max(0.1);
        let cursor_y = (pt.y as f64 - overlay.window_y as f64) / overlay.scale.max(0.1);
        let hit = hit_test(&layout, cursor_x, cursor_y);

        let (target_id, region_index) = {
            let mut state = self.context.lock();
            if let Some(session) = state.session.as_mut() {
                let region = hit.region;
                session.layout = layout.clone();
                (
                    region.and_then(|index| session.targets.get(index).map(|target| target.id.clone())),
                    region,
                )
            } else {
                (None, None)
            }
        };

        let payload = DragOverPayload {
            cursor_x,
            cursor_y,
            region_index,
            target_id,
            angle: hit.angle,
            distance: hit.distance,
            over_center: hit.over_center,
        };
        self.context.emit("drag-over", payload);

        unsafe {
            *pdweffect = DROPEFFECT_COPY;
        }
        Ok(())
    }

    fn DragLeave(&self) -> windows::core::Result<()> {
        self.context.lock().session = None;
        self.context.emit("drag-leave", ());
        Ok(())
    }

    fn Drop(
        &self,
        _pdataobj: Ref<IDataObject>,
        _grfkeystate: MODIFIERKEYS_FLAGS,
        pt: &POINTL,
        pdweffect: *mut DROPEFFECT,
    ) -> windows::core::Result<()> {
        unsafe {
            *pdweffect = DROPEFFECT_NONE;
        }

        let session = self.context.lock().session.take();
        let Some(session) = session else {
            hide_shield();
            self.context.emit(
                "drag-invalid",
                DragMessage {
                    message: "Arquivo não reconhecido.".to_string(),
                    cursor_x: pt.x as f64,
                    cursor_y: pt.y as f64,
                },
            );
            return Ok(());
        };

        if self.context.cancelled.load(Ordering::SeqCst) {
            hide_shield();
            self.context.emit("drag-cancel", ());
            return Ok(());
        }

        let cursor_x = (pt.x as f64 - session.overlay.window_x as f64)
            / session.overlay.scale.max(0.1);
        let cursor_y = (pt.y as f64 - session.overlay.window_y as f64)
            / session.overlay.scale.max(0.1);
        let hit = hit_test(&session.layout, cursor_x, cursor_y);

        let Some(region_index) = hit.region else {
            // O drop terminou, mas fora de uma região: mantenha o original
            // intacto e mostre uma mensagem curta. O frontend fecha depois.
            self.context.dropped.store(true, Ordering::SeqCst);
            hide_shield();
            self.context.emit(
                "drop-invalid",
                DragMessage {
                    message: if hit.over_center {
                        "Mova até um formato e solte sobre ele.".to_string()
                    } else {
                        "Solte sobre um dos formatos da roda.".to_string()
                    },
                    cursor_x,
                    cursor_y,
                },
            );
            return Ok(());
        };

        let Some(target) = session.targets.get(region_index).cloned() else {
            hide_shield();
            return Ok(());
        };

        let request = ConversionRequest {
            source_path: session.file.path.clone(),
            target_format: target.id.clone(),
        };

        match conversion::start_conversion(self.context.app.clone(), request) {
            Ok(conversion_id) => {
                self.context.dropped.store(true, Ordering::SeqCst);
                hide_shield();
                if let Err(error) = show_hud_at(&self.context.app, session.layout.center_x, session.layout.center_y) {
                    warn!("Não foi possível mostrar o HUD: {}", error);
                }
                let payload = DropPayload {
                    file: session.file,
                    target: target.clone(),
                    conversion_id,
                    layout: session.layout,
                    overlay: session.overlay,
                    cursor_x,
                    cursor_y,
                };
                self.context.emit("drop", payload);
                unsafe {
                    *pdweffect = DROPEFFECT_COPY;
                }
            }
            Err(message) => {
                hide_shield();
                self.context.dropped.store(true, Ordering::SeqCst);
                self.context.emit(
                    "drop-error",
                    DragMessage {
                        message,
                        cursor_x,
                        cursor_y,
                    },
                );
            }
        }

        Ok(())
    }
}

unsafe fn extract_hdrop(data_object: &IDataObject) -> Option<(u32, String)> {
    let format = FORMATETC {
        cfFormat: CF_HDROP.0,
        ptd: std::ptr::null_mut(),
        dwAspect: DVASPECT_CONTENT.0,
        lindex: -1,
        tymed: TYMED_HGLOBAL.0 as u32,
    };

    let mut medium = data_object.GetData(&format).ok()?;
    let hdrop = HDROP(medium.u.hGlobal.0);

    let count = DragQueryFileW(hdrop, u32::MAX, None);
    if count == 0 {
        ReleaseStgMedium(&mut medium);
        return None;
    }

    let length = DragQueryFileW(hdrop, 0, None);
    let mut buffer = vec![0u16; length as usize + 1];
    let copied = DragQueryFileW(hdrop, 0, Some(&mut buffer));
    ReleaseStgMedium(&mut medium);

    if copied == 0 {
        return None;
    }

    Some((count, String::from_utf16_lossy(&buffer[..copied as usize])))
}

fn emit_invalid(context: &Arc<DropContext>, message: &str, pt: &POINTL) {
    warn!("Drag inválido: {}", message);
    let (window_rect, scale) = {
        let state = context.lock();
        (state.window_rect, state.scale.max(0.1))
    };
    context.emit(
        "drag-invalid",
        DragMessage {
            message: message.to_string(),
            cursor_x: (pt.x as f64 - window_rect.x as f64) / scale,
            cursor_y: (pt.y as f64 - window_rect.y as f64) / scale,
        },
    );
}

// ---------------------------------------------------------------------------
// Geometria orgânica
// ---------------------------------------------------------------------------

struct Hit {
    region: Option<usize>,
    angle: f64,
    distance: f64,
    over_center: bool,
}

/// Diâmetro físico desejado para a roda, em pixels da tela. A faixa de
/// 240–300 px mantém o seletor compacto em qualquer escala de DPI: o valor é
/// convertido para unidades CSS dividindo pela escala do monitor.
fn wheel_target_physical(count: usize) -> f64 {
    match count.max(1) {
        1..=6 => 260.0,
        7..=8 => 272.0,
        _ => 284.0,
    }
}

fn visual_radius(outer: f64) -> f64 {
    outer * 1.18 + 16.0
}

// Precisam acompanhar as constantes de geometria em FormatWheel.tsx.
fn orbit_rim_gap(outer: f64) -> f64 {
    (outer * 0.058).max(3.8)
}

fn orbit_hub_gap(outer: f64) -> f64 {
    (outer * 0.07).max(4.9)
}

fn orbit_panel_gap(outer: f64) -> f64 {
    (outer * 0.054).max(4.2)
}

fn make_layout(
    cursor_x: f64,
    cursor_y: f64,
    window_width: f64,
    window_height: f64,
    count: usize,
    scale: f64,
) -> WheelLayout {
    let count = count.max(1);
    let n = count as f64;
    let available = window_width.min(window_height).max(1.0);
    let scale = scale.max(0.5);

    // O raio é calculado a partir do diâmetro físico alvo. Assim a roda
    // mantém o mesmo tamanho aparente em monitores a 100%, 125%, 150% etc.
    let outer = (wheel_target_physical(count) * 0.5 / scale)
        .max(48.0)
        .min(available * 0.42);
    let inner = outer * 0.35;
    let visual = visual_radius(outer);

    let center_x = if window_width > visual * 2.0 {
        cursor_x.clamp(visual, window_width - visual)
    } else {
        window_width / 2.0
    };
    let center_y = if window_height > visual * 2.0 {
        cursor_y.clamp(visual, window_height - visual)
    } else {
        window_height / 2.0
    };

    let rotation = -std::f64::consts::FRAC_PI_2 - std::f64::consts::PI / n;

    // Costuras compartilhadas e claramente curvas. O sinal alternado evita
    // que todas as pétalas se desloquem para o mesmo lado, preservando o
    // equilíbrio radial sem transformar o menu em fatias de pizza.
    let seam_bends: Vec<f64> = (0..count)
        .map(|index| {
            let i = index as f64;
            let magnitude = 0.068 + 0.026 * (i * 1.73 + 0.9).sin().abs();
            let sign = if (i * 1.7 + 0.6).sin() >= 0.0 {
                1.0
            } else {
                -1.0
            };
            sign * magnitude
        })
        .collect();

    WheelLayout {
        center_x,
        center_y,
        outer_radius: outer,
        inner_radius: inner,
        rotation,
        seam_bends,
        // Silhueta perfeitamente circular; a organicidade fica nas costuras.
        outer_wave: vec![0.0, 0.0, 0.0],
        outer_phase: vec![0.7, 2.1, 4.3],
        inner_wave: vec![0.0, 0.0],
        inner_phase: vec![1.3, 3.8],
    }
}

fn outer_radius_at(layout: &WheelLayout, angle: f64) -> f64 {
    let wave = |index: usize| layout.outer_wave.get(index).copied().unwrap_or(0.0);
    let phase = |index: usize| layout.outer_phase.get(index).copied().unwrap_or(0.0);
    layout.outer_radius
        * (1.0
            + wave(0) * (3.0 * angle + phase(0)).sin()
            + wave(1) * (5.0 * angle + phase(1)).sin()
            + wave(2) * (7.0 * angle + phase(2)).sin())
}

fn inner_radius_at(layout: &WheelLayout, angle: f64) -> f64 {
    let wave = |index: usize| layout.inner_wave.get(index).copied().unwrap_or(0.0);
    let phase = |index: usize| layout.inner_phase.get(index).copied().unwrap_or(0.0);
    layout.inner_radius
        * (1.0
            + wave(0) * (2.0 * angle + phase(0)).sin()
            + wave(1) * (4.0 * angle + phase(1)).sin())
}

fn seam_angle(layout: &WheelLayout, index: usize, normalized_radius: f64) -> f64 {
    let count = layout.seam_bends.len().max(1) as f64;
    let base = layout.rotation + index as f64 * TAU / count;
    let bend = layout.seam_bends.get(index).copied().unwrap_or(0.0);
    base + bend * (std::f64::consts::PI * normalized_radius).sin()
}

fn hit_test(layout: &WheelLayout, x: f64, y: f64) -> Hit {
    let dx = x - layout.center_x;
    let dy = y - layout.center_y;
    let distance = (dx * dx + dy * dy).sqrt();
    let angle = dy.atan2(dx).rem_euclid(TAU);

    let inner = inner_radius_at(layout, angle);
    let outer = outer_radius_at(layout, angle);
    let region_inner = inner + orbit_hub_gap(outer);
    let region_outer = outer - orbit_rim_gap(outer);

    if distance < inner * 0.94 {
        return Hit {
            region: None,
            angle,
            distance,
            over_center: true,
        };
    }
    if distance < region_inner || distance > region_outer {
        return Hit {
            region: None,
            angle,
            distance,
            over_center: false,
        };
    }

    if layout.seam_bends.len() <= 1 {
        return Hit {
            region: Some(0),
            angle,
            distance,
            over_center: false,
        };
    }

    let t = ((distance - inner) / (outer - inner).max(1.0)).clamp(0.0, 1.0);
    let count = layout.seam_bends.len();
    let mut seams: Vec<(f64, usize)> = (0..count)
        .map(|index| {
            (
                seam_angle(layout, index, t).rem_euclid(TAU),
                index,
            )
        })
        .collect();
    seams.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));

    // A região i fica entre a costura i e a costura i+1. A costura anterior
    // em sentido anti-horário define a região sob o cursor.
    let mut previous = seams[count - 1].1;
    for (seam, index) in &seams {
        if *seam <= angle + 1e-9 {
            previous = *index;
        } else {
            break;
        }
    }

    // A pétala visual começa depois da costura; o vão entre pétalas é uma
    // zona neutra, coerente com os caminhos SVG clicáveis.
    let gap_angle = (orbit_panel_gap(outer) * 0.5) / distance.max(1.0);
    let over_seam = seams.iter().any(|(seam, _)| {
        let delta = (angle - seam).rem_euclid(TAU);
        delta.min(TAU - delta) < gap_angle
    });
    if over_seam {
        return Hit {
            region: None,
            angle,
            distance,
            over_center: false,
        };
    }

    Hit {
        region: Some(previous),
        angle,
        distance,
        over_center: false,
    }
}

// ---------------------------------------------------------------------------
// Captura de fundo
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Watcher de Shift + arrastar
// ---------------------------------------------------------------------------

fn spawn_watcher(context: Arc<DropContext>) {
    std::thread::Builder::new()
        .name("shiftconvert-drag-watch".to_string())
        .spawn(move || watch_loop(context))
        .map(|_| ())
        .unwrap_or_else(|error| error!("Falha ao iniciar watcher de arraste: {}", error));
}

fn watch_loop(context: Arc<DropContext>) {
    let mut previous_left = false;
    let mut start_point = POINT::default();
    let mut start_root = HWND(std::ptr::null_mut());
    let mut start_view = HWND(std::ptr::null_mut());
    let mut candidate = false;
    let mut moved = false;
    let mut armed = false;
    let mut suppress_until_release = false;

    info!("Watcher de Shift + arrastar iniciado.");

    loop {
        std::thread::sleep(Duration::from_millis(22));

        if ONBOARDING_ACTIVE.load(Ordering::SeqCst) {
            if armed {
                finish_drag(&context);
                armed = false;
            }
            previous_left = false;
            moved = false;
            candidate = false;
            suppress_until_release = false;
            continue;
        }

        let left = key_down(VK_LBUTTON.0 as i32);
        let shift = key_down(VK_SHIFT.0 as i32);
        let escape = key_down(VK_ESCAPE.0 as i32);

        if !left {
            if armed {
                // Dá tempo para o OLE entregar o Drop antes de esconder o
                // escudo. Sem isso, o Windows poderia encerrar o arraste
                // antes de o alvo receber a soltura.
                std::thread::sleep(Duration::from_millis(140));
                finish_drag(&context);
                armed = false;
            }
            previous_left = false;
            moved = false;
            candidate = false;
            suppress_until_release = false;
            continue;
        }

        if !previous_left {
            let mut point = POINT::default();
            unsafe {
                let _ = GetCursorPos(&mut point);
            }
            start_point = point;
            let view = unsafe { WindowFromPoint(point) };
            start_view = view;
            start_root = unsafe { GetAncestor(view, GA_ROOT) };
            candidate = is_explorer_root(start_root) && is_file_view(view);
            moved = false;
            suppress_until_release = false;
            previous_left = true;
        }

        let mut point = POINT::default();
        unsafe {
            let _ = GetCursorPos(&mut point);
        }

        if !moved {
            let dx = point.x - start_point.x;
            let dy = point.y - start_point.y;
            if dx * dx + dy * dy > 8 * 8 {
                moved = true;
            }
        }

        if !armed && !suppress_until_release && shift && moved && candidate {
            let foreground = unsafe { GetForegroundWindow() };
            let foreground_root = unsafe { GetAncestor(foreground, GA_ROOT) };
            if is_explorer_root(foreground_root) || is_explorer_root(start_root) {
                match begin_drag(&context, point) {
                    Ok(()) => {
                        armed = true;
                        info!(
                            "Shift + arrastar detectado em {:?} / {:?}",
                            start_root.0, start_view.0
                        );
                    }
                    Err(error) => {
                        warn!("Não foi possível armar o drop target: {}", error);
                        suppress_until_release = true;
                    }
                }
            }
        }

        if armed && escape && !context.cancelled.load(Ordering::SeqCst) {
            context.cancelled.store(true, Ordering::SeqCst);
            context.emit("drag-cancel", ());
            // A janela da roda pode sumir, mas o escudo continua cobrindo a
            // área de trabalho até soltar o botão. Assim o Explorer não move
            // o arquivo para uma pasta por engano.
            overlay::hide_window(&context.app);
            debug!("Arraste cancelado com Esc; escudo mantido até soltar.");
        }
    }
}

fn begin_drag(context: &Arc<DropContext>, cursor: POINT) -> Result<(), String> {
    let app = &context.app;
    let work_area = screen::work_area_for_point(cursor.x, cursor.y);
    let overlay_rect = ScreenRect::new(
        work_area.x,
        work_area.y,
        work_area.width,
        work_area.height,
    );

    // Esconde o cartão inicial e limpa o conteúdo antes de expandir, para não
    // piscar o prompt em tela cheia.
    overlay::hide_window(app);
    context.emit("drag-armed", ());
    std::thread::sleep(Duration::from_millis(40));

    overlay::position_window(app, overlay_rect)?;
    let scale = screen::dpi_scale_for_point(cursor.x, cursor.y);
    debug!(
        "begin_drag: área {}x{}@{},{} | janela {}x{}@{},{} | escala {:.3}",
        work_area.width,
        work_area.height,
        work_area.x,
        work_area.y,
        overlay_rect.width,
        overlay_rect.height,
        overlay_rect.x,
        overlay_rect.y,
        scale
    );

    {
        let mut state = context.lock();
        state.session = None;
        state.window_rect = overlay_rect;
        state.work_area = work_area;
        state.scale = scale;
    }

    context.cancelled.store(false, Ordering::SeqCst);
    context.dropped.store(false, Ordering::SeqCst);
    ACTIVE.store(true, Ordering::SeqCst);

    if let Err(error) = overlay::show_no_activate(app) {
        ACTIVE.store(false, Ordering::SeqCst);
        return Err(error);
    }
    if let Err(error) = show_shield(overlay_rect) {
        ACTIVE.store(false, Ordering::SeqCst);
        overlay::hide_window(app);
        return Err(error);
    }
    Ok(())
}

fn finish_drag(context: &Arc<DropContext>) {
    hide_shield();
    ACTIVE.store(false, Ordering::SeqCst);

    let dropped = context.dropped.swap(false, Ordering::SeqCst);
    let cancelled = context.cancelled.swap(false, Ordering::SeqCst);
    context.lock().session = None;

    if !dropped || cancelled {
        overlay::hide_window(&context.app);
    }
}

fn key_down(key: i32) -> bool {
    unsafe { (GetAsyncKeyState(key) as u16 & 0x8000) != 0 }
}

fn is_explorer_root(hwnd: HWND) -> bool {
    let class = class_name(hwnd);
    matches!(
        class.as_str(),
        "CabinetWClass" | "ExploreWClass" | "Progman" | "WorkerW"
    )
}

fn is_file_view(hwnd: HWND) -> bool {
    let mut current = hwnd;
    for _ in 0..12 {
        let class = class_name(current);
        if matches!(
            class.as_str(),
            "SHELLDLL_DefView" | "DirectUIHWND" | "SysListView32" | "UIItemsView"
        ) {
            return true;
        }
        if matches!(class.as_str(), "CabinetWClass" | "ExploreWClass" | "Progman" | "WorkerW") {
            return false;
        }
        let Ok(parent) = (unsafe { GetParent(current) }) else {
            break;
        };
        if parent.0.is_null() {
            break;
        }
        current = parent;
    }
    false
}

fn class_name(hwnd: HWND) -> String {
    let mut buffer = [0u16; 256];
    let length = unsafe { GetClassNameW(hwnd, &mut buffer) };
    if length <= 0 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..length as usize])
}

// ---------------------------------------------------------------------------
// Janela escudo (OLE drop target)
// ---------------------------------------------------------------------------

fn show_shield(rect: ScreenRect) -> Result<(), String> {
    let Some(context) = DROP_CONTEXT.get() else {
        return Err("contexto do drop target ausente".to_string());
    };
    let hwnd = HWND(context.shield_hwnd.load(Ordering::SeqCst) as *mut c_void);
    if hwnd.0.is_null() {
        return Err("janela escudo ausente".to_string());
    }

    unsafe {
        SetWindowPos(
            hwnd,
            Some(HWND_TOPMOST),
            rect.x,
            rect.y,
            rect.width,
            rect.height,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        )
        .map_err(|error| format!("não foi possível mostrar o escudo: {error}"))?;
        let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    }
    Ok(())
}

fn hide_shield() {
    let Some(context) = DROP_CONTEXT.get() else {
        return;
    };
    let hwnd = HWND(context.shield_hwnd.load(Ordering::SeqCst) as *mut c_void);
    if !hwnd.0.is_null() {
        unsafe {
            let _ = ShowWindow(hwnd, SW_HIDE);
        }
    }
}

pub fn force_hide(app: &AppHandle) {
    hide_shield();
    ACTIVE.store(false, Ordering::SeqCst);
    if let Some(context) = DROP_CONTEXT.get() {
        context.cancelled.store(false, Ordering::SeqCst);
        context.dropped.store(false, Ordering::SeqCst);
        context.lock().session = None;
    }
    overlay::hide_window(app);
}

// ---------------------------------------------------------------------------
// HUD e seletor nativo
// ---------------------------------------------------------------------------

pub fn show_hud_at(app: &AppHandle, anchor_x: f64, anchor_y: f64) -> Result<HudInfo, String> {
    let hwnd = overlay::hwnd_of(app)?;
    let current = overlay::window_rect(app)?;
    let scale = screen::dpi_scale_for_window(hwnd).max(0.5);
    let area = screen::work_area_for_window(hwnd);

    let hud_width_css = 420.0;
    let hud_height_css = 204.0;
    let width = (hud_width_css * scale).round() as i32;
    let height = (hud_height_css * scale).round() as i32;
    let x = current.x + (anchor_x * scale).round() as i32 - width / 2;
    let y = current.y + (anchor_y * scale).round() as i32 - height / 2;

    let rect = ScreenRect::new(x, y, width, height)
        .clamp_to(area)
        .ok_or_else(|| "não foi possível enquadrar o HUD".to_string())?;
    overlay::set_window_rect(app, rect, false)?;

    Ok(HudInfo {
        width: hud_width_css,
        height: hud_height_css,
    })
}

pub async fn pick_file(app: AppHandle) -> Result<PickerResponse, String> {
    overlay::hide_window(&app);

    let picked = tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("Escolher arquivo para converter")
            .pick_file()
    })
    .await
    .map_err(|error| format!("diálogo indisponível: {error}"))?;

    let Some(path) = picked else {
        let _ = overlay::show_prompt(&app);
        return Ok(PickerResponse {
            cancelled: true,
            wheel: None,
            message: None,
        });
    };

    let file = match file_info::inspect_file(&path.to_string_lossy()) {
        Ok(file) => file,
        Err(message) => {
            let _ = overlay::show_prompt(&app);
            return Ok(PickerResponse {
                cancelled: false,
                wheel: None,
                message: Some(message),
            });
        }
    };

    if !file.is_readable {
        let _ = overlay::show_prompt(&app);
        return Ok(PickerResponse {
            cancelled: false,
            wheel: None,
            message: Some("Sem permissão para ler este arquivo.".to_string()),
        });
    }

    let targets = file_info::supported_targets(std::path::Path::new(&file.path));
    if targets.is_empty() {
        let _ = overlay::show_prompt(&app);
        return Ok(PickerResponse {
            cancelled: false,
            wheel: None,
            message: Some("Nenhuma conversão real disponível para este arquivo.".to_string()),
        });
    }

    let hwnd = overlay::hwnd_of(&app)?;
    let area = screen::work_area_for_window(hwnd);
    let scale = screen::dpi_scale_for_window(hwnd).max(0.5);
    let available_css = (area.width.min(area.height) as f64 / scale) - 28.0;
    let outer_css = (wheel_target_physical(targets.len()) * 0.5 / scale)
        .max(48.0)
        .min(available_css * 0.42);
    let window_css = (visual_radius(outer_css) * 2.0)
        .clamp(170.0, 520.0)
        .min(available_css.max(170.0));
    let window_px = (window_css * scale).round() as i32;
    let rect = ScreenRect::new(
        area.x + (area.width - window_px) / 2,
        area.y + (area.height - window_px) / 2,
        window_px,
        window_px,
    );

    overlay::position_window(&app, rect)?;

    let layout = make_layout(
        window_css / 2.0,
        window_css / 2.0,
        window_css,
        window_css,
        targets.len(),
        scale,
    );
    let overlay = OverlayContext {
        window_x: rect.x,
        window_y: rect.y,
        window_width: rect.width,
        window_height: rect.height,
        work_x: area.x,
        work_y: area.y,
        work_width: area.width,
        work_height: area.height,
        scale,
    };
    let backdrop: Option<Backdrop> = None;

    overlay::set_window_rect(&app, rect, true)?;

    Ok(PickerResponse {
        cancelled: false,
        wheel: Some(WheelPayload {
            mode: "picker".to_string(),
            file,
            targets,
            layout,
            backdrop,
            overlay,
        }),
        message: None,
    })
}

/// Seletor usado exclusivamente pela etapa "Try it yourself" do onboarding.
///
/// A janela é escondida durante o diálogo nativo para que ele não fique atrás
/// da janela always-on-top. O estado do React permanece montado e a mesma
/// janela do onboarding é restaurada ao final.
pub async fn pick_file_for_onboarding(
    app: AppHandle,
) -> Result<Option<OnboardingFileInfo>, String> {
    overlay::hide_window(&app);

    let picked = tauri::async_runtime::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("Escolher arquivo para experimentar")
            .pick_file()
    })
    .await
    .map_err(|error| format!("diálogo indisponível: {error}"));

    let restore = || overlay::show_onboarding(&app).map(|_| ());

    let picked = match picked {
        Ok(path) => path,
        Err(error) => {
            restore()?;
            return Err(error);
        }
    };

    let Some(path) = picked else {
        restore()?;
        return Ok(None);
    };

    let file = match file_info::inspect_file(&path.to_string_lossy()) {
        Ok(file) => file,
        Err(message) => {
            restore()?;
            return Err(message);
        }
    };

    if !file.is_readable {
        restore()?;
        return Err("Sem permissão para ler este arquivo.".to_string());
    }

    let targets = file_info::supported_targets(std::path::Path::new(&file.path));
    if targets.is_empty() {
        restore()?;
        return Err("Nenhuma conversão real disponível para este arquivo.".to_string());
    }

    restore()?;
    Ok(Some(OnboardingFileInfo { file, targets }))
}
