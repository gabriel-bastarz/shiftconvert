# Avisos de terceiros

## FFmpeg

O instalador do ShiftConvert inclui o **FFmpeg 9.0.2 essentials build para
Windows x64**, distribuído como um executável separado em `ffmpeg/ffmpeg.exe`.

- Licença: **GPLv3**
- Texto da licença: [`licenses/GPL-3.0.txt`](licenses/GPL-3.0.txt)
- Pacote binário: <https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-9.0.2-essentials_build.7z>
- SHA-256 do pacote: `4705843CCAAF54257C16AD90F3E952ECE33C17DF964ECF7BFDBB0F49C7171077`
- SHA-256 do `ffmpeg.exe`: `3256173F3F8BFFD7DF12227C68ADF68025EDB1832273A9530688A7BB1ED8EDEC`
- Código-fonte correspondente: <https://github.com/FFmpeg/FFmpeg/commit/946fcce07b>
- Scripts de build: <https://github.com/GyanD/codexffmpeg>

O FFmpeg é executado pelo ShiftConvert como um processo externo, com argumentos
separados. O ShiftConvert não modifica o binário do FFmpeg. O texto completo da
GPLv3 e os metadados do build também são incluídos na pasta `ffmpeg/` do
instalador.

## English

The ShiftConvert installer bundles **FFmpeg 9.0.2 essentials build for Windows
x64** as a separate executable at `ffmpeg/ffmpeg.exe`.

- License: **GPLv3**
- License text: [`licenses/GPL-3.0.txt`](licenses/GPL-3.0.txt)
- Binary package: <https://www.gyan.dev/ffmpeg/builds/packages/ffmpeg-9.0.2-essentials_build.7z>
- Package SHA-256: `4705843CCAAF54257C16AD90F3E952ECE33C17DF964ECF7BFDBB0F49C7171077`
- `ffmpeg.exe` SHA-256: `3256173F3F8BFFD7DF12227C68ADF68025EDB1832273A9530688A7BB1ED8EDEC`
- Corresponding source: <https://github.com/FFmpeg/FFmpeg/commit/946fcce07b>
- Build scripts: <https://github.com/GyanD/codexffmpeg>

FFmpeg runs as a separate process. ShiftConvert does not modify the FFmpeg
binary. The full GPLv3 text and build metadata are also included in the
installer's `ffmpeg/` folder.
