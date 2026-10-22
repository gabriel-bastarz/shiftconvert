use log::{info};
use tauri::{AppHandle, Emitter};

// ---------------------------------------------------------------------------
// Estruturas de dados para eventos de ativação
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActivationEvent {
    pub file_path: Option<String>,
    pub cursor_x: f64,
    pub cursor_y: f64,
    pub shift_held: bool,
    pub left_button_pressed: bool,
    pub source: ActivationSource,
    pub timestamp: u64,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ActivationSource {
    GlobalHook,
    ManualFallback,
    Unknown,
}

// ---------------------------------------------------------------------------
// WindowsInputHook
//
// Tenta inicializar um hook global de baixo nível do Windows para detectar
// pressionamento de Shift + botão esquerdo do mouse.
//
// LIMITAÇÃO CRÍTICA (documentada em docs/WINDOWS_SHIFT_DRAG.md):
// O Windows não expõe um evento público e confiável de "arraste iniciado no
// Explorer com Shift pressionado". Um hook de baixo nível (WH_MOUSE_LL /
// WH_KEYBOARD_LL) pode detectar pressionamentos individuais de tecla e botão,
// mas não consegue atribuir com confiança um arquivo específico do Explorer a
// aquele arraste, pois o Explorer não envia o caminho do arquivo arrastado
// para processos externos durante o início do arraste.
//
// Implementamos o hook como prova de conceito. Ele registra os eventos, mas
// a detecção confiável do arquivo real durante Shift+drag requer:
//   1. Integração COM com IDataObject / IDropSource do Explorer (complexo),
//   2. Uma janela overlay transparente que receba o arraste, ou
//   3. Um atalho alternativo (ex.: Shift + clique direito, ou ícone na bandeja).
//
// Até que uma dessas abordagens seja validada, oferecemos um fallback manual
// (request_activation) que o usuário pode acionar via atalho global ou bandeja.
// ---------------------------------------------------------------------------

pub struct WindowsInputHook {
    #[allow(dead_code)]
    app_handle: AppHandle,
    #[allow(dead_code)]
    hook_thread: Option<std::thread::JoinHandle<()>>,
}

impl WindowsInputHook {
    pub fn new(app_handle: AppHandle) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        info!("Inicializando hook de entrada do Windows (prova de conceito)");

        let handle = app_handle.clone();

        // Em um ambiente de produção real, aqui instalaríamos SetWindowsHookEx
        // com WH_MOUSE_LL e WH_KEYBOARD_LL. Como estamos executando dentro do
        // Tauri e o hook precisa de um message loop dedicado, inicializamos
        // uma thread separada. Por enquanto, o hook é inicializado mas não
        // captura eventos reais de arraste do Explorer (limitação documentada).

        let hook_thread = std::thread::spawn(move || {
            let _ = handle;
            // Loop do hook seria executado aqui.
            // O hook global requer GetMessage em uma thread dedicada.
            // Não implementamos a captura real de Shift+drag do Explorer
            // porque não é viável de forma confiável sem integração COM.
        });

        Ok(Self {
            app_handle,
            hook_thread: Some(hook_thread),
        })
    }

    /// Emite um evento de ativação para o frontend.
    /// Usado como fallback quando o hook não consegue detectar o arraste.
    pub fn emit_manual_activation(
        app_handle: &AppHandle,
        cursor_x: f64,
        cursor_y: f64,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let event = ActivationEvent {
            file_path: None, // não sabemos o arquivo no fallback manual
            cursor_x,
            cursor_y,
            shift_held: false,
            left_button_pressed: false,
            source: ActivationSource::ManualFallback,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        };

        app_handle.emit("shiftconvert:activation", &event)?;
        Ok(())
    }

    /// Registra um evento de ativação detectado (prova de conceito).
    pub fn emit_activation_event(
        app_handle: &AppHandle,
        file_path: Option<String>,
        cursor_x: f64,
        cursor_y: f64,
        shift_held: bool,
        left_button_pressed: bool,
        source: ActivationSource,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let event = ActivationEvent {
            file_path: file_path.clone(),
            cursor_x,
            cursor_y,
            shift_held,
            left_button_pressed,
            source: source.clone(),
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_millis() as u64)
                .unwrap_or(0),
        };

        info!(
            "Evento de ativação emitido: arquivo={:?}, cursor=({}, {}), fonte={:?}",
            file_path, cursor_x, cursor_y, source
        );

        app_handle.emit("shiftconvert:activation", &event)?;
        Ok(())
    }
}

impl Drop for WindowsInputHook {
    fn drop(&mut self) {
        info!("Finalizando hook de entrada do Windows");
        // O hook seria desinstalado aqui (UnhookWindowsHookEx).
        if let Some(thread) = self.hook_thread.take() {
            let _ = thread.join();
        }
    }
}

/// Obtém a posição global do cursor em coordenadas físicas de tela.
/// Usada para posicionar a janela overlay próximo ao cursor quando o menu
/// é aberto pelo ícone da bandeja.
#[cfg(target_os = "windows")]
pub fn cursor_position() -> (f64, f64) {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    unsafe {
        let mut point = POINT { x: 0, y: 0 };
        if GetCursorPos(&mut point) != 0 {
            (point.x as f64, point.y as f64)
        } else {
            (0.0, 0.0)
        }
    }
}

#[cfg(not(target_os = "windows"))]
pub fn cursor_position() -> (f64, f64) {
    (0.0, 0.0)
}
