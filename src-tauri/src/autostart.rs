use std::env;
use std::path::Path;

use windows_registry::{Type, CURRENT_USER};

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const APPROVED_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run";
const VALUE_NAME: &str = "ShiftConvert";
const ENABLED_VALUE: [u8; 12] = [
    0x02, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
];

fn command_for(executable: &Path) -> String {
    format!("\"{}\"", executable.display())
}

pub fn is_enabled() -> bool {
    let Ok(executable) = env::current_exe() else {
        return false;
    };
    let expected = command_for(&executable);

    CURRENT_USER
        .open(RUN_KEY)
        .and_then(|key| key.get_string(VALUE_NAME))
        .map(|value| value.trim().eq_ignore_ascii_case(&expected))
        .unwrap_or(false)
}

pub fn enable() -> Result<(), String> {
    let executable = env::current_exe()
        .map_err(|error| format!("não foi possível localizar o executável: {error}"))?;
    let command = command_for(&executable);

    CURRENT_USER
        .create(RUN_KEY)
        .and_then(|key| key.set_string(VALUE_NAME, command))
        .map_err(|error| format!("não foi possível ativar a inicialização automática: {error}"))?;

    // Remove o bloqueio que o Gerenciador de Tarefas do Windows mantém quando
    // o usuário desativa uma entrada de inicialização.
    if let Ok(key) = CURRENT_USER.options().write().open(APPROVED_KEY) {
        let _ = key.set_bytes(VALUE_NAME, Type::Bytes, &ENABLED_VALUE);
    }

    Ok(())
}

pub fn disable() -> Result<(), String> {
    if !is_enabled() {
        return Ok(());
    }

    CURRENT_USER
        .options()
        .write()
        .open(RUN_KEY)
        .and_then(|key| key.remove_value(VALUE_NAME))
        .map_err(|error| {
            format!("não foi possível desativar a inicialização automática: {error}")
        })?;

    if let Ok(key) = CURRENT_USER.options().write().open(APPROVED_KEY) {
        let _ = key.remove_value(VALUE_NAME);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_quote_paths_with_spaces() {
        assert_eq!(
            command_for(Path::new(r"C:\Program Files\ShiftConvert.exe")),
            r#""C:\Program Files\ShiftConvert.exe""#
        );
    }
}
