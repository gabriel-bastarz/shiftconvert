<div align="center">
  <img src="src-tauri/icons/icon.png" width="104" alt="ShiftConvert" />

  <h1>ShiftConvert</h1>

  <p><strong>Shift + arraste. Escolha o formato. Converta.</strong></p>

  <p>
    Conversor de arquivos local para Windows com uma interface radial que aparece
    diretamente no Explorer, durante o arraste.
  </p>

  <p>
    <img src="https://img.shields.io/badge/Windows-10%20%7C%2011-0078D4?style=flat-square" alt="Windows 10 e 11" />
    <img src="https://img.shields.io/badge/Tauri-2-24C8DB?style=flat-square" alt="Tauri 2" />
    <img src="https://img.shields.io/badge/Rust-stable-000000?style=flat-square" alt="Rust" />
    <img src="https://img.shields.io/badge/React-18-61DAFB?style=flat-square" alt="React 18" />
    <img src="https://img.shields.io/badge/status-alpha-F59E0B?style=flat-square" alt="Status alpha" />
  </p>

  <p>
    <a href="https://github.com/gabriel-bastarz/shiftconvert/releases/latest">Download</a>
    ·
    <a href="#português">Português</a>
    ·
    <a href="#english">English</a>
    ·
    <a href="docs/WINDOWS_SHIFT_DRAG.md">Documentação técnica</a>
  </p>
</div>

---

## Português

O ShiftConvert transforma o gesto de arrastar do Windows Explorer em uma interface de conversão. Segure **Shift**, arraste um arquivo e uma roda radial aparece no cursor. Passe o mouse sobre o formato desejado e solte para converter.

Não há janela principal nem upload: o processamento acontece inteiramente na máquina local. O arquivo original é aberto somente para leitura e nunca é movido, alterado ou apagado.

### Destaques

- **Shift + arrastar no Explorer** abre a roda de formatos no cursor.
- **Apenas conversões reais** aparecem; cada opção representa uma conversão suportada.
- **O original é preservado** — o drop é tratado como cópia e a saída é gravada ao lado do arquivo.
- **Sem sobrescrita silenciosa** — nomes existentes recebem sufixo ` (1)`, ` (2)`, etc.
- **Interface Orbit Minimal** — superfícies opacas em grafite, pétalas arredondadas e destaque azul elétrico.
- **Escala e múltiplos monitores** — posicionamento e tamanho usam o monitor do cursor.
- **Funciona offline** — sem telemetria, sem upload e sem serviços externos.
- **Alternativas de acesso** — ícone na bandeja e botão para escolher um arquivo manualmente.
- **Início com o Windows** — opção no menu da bandeja, com estado marcado/desmarcado.
- **FFmpeg opcional** — quando disponível, habilita as conversões de vídeo.
- **Cancelamento seguro** — arquivos temporários são removidos e a origem permanece intacta.

### Como usar

1. Baixe o pacote mais recente na página de [Releases](https://github.com/gabriel-bastarz/shiftconvert/releases/latest).
2. Extraia o ZIP para uma pasta de sua preferência e execute `ShiftConvert.exe`.
3. No Explorer, segure **Shift**, arraste um arquivo e passe o mouse sobre o formato desejado.
4. Solte sobre a opção para converter. O resultado é salvo ao lado do arquivo original.

O aplicativo fica na bandeja do Windows. Clique com o botão direito no ícone para abrir a interface, escolher um arquivo ou ativar **Iniciar com o Windows**.

> Esta versão é um executável portátil: não há instalador nem assinatura de código. O Windows SmartScreen pode exibir um aviso na primeira execução. Se o executável for movido depois de ativar o início automático, reative a opção no menu da bandeja.

### Formatos suportados

| Origem | Destinos oferecidos |
|---|---|
| JPG/JPEG, PNG, GIF, BMP, WEBP, TIFF/TIF, ICO | demais formatos de imagem + ZIP |
| TXT | MD, HTML + ZIP |
| MD/Markdown | TXT, HTML + ZIP |
| HTML/HTM | TXT + ZIP |
| MP4, MKV, WEBM, AVI, MOV, WMV, FLV, M4V, MPG/MPEG | MP4, WEBM, MKV, GIF animado (exceto o mesmo formato) + ZIP |
| DOCX | PDF, HTML, TXT + ZIP |
| PDF | DOCX, HTML, TXT + ZIP |
| DOC, ODT, RTF, CSV, JSON, XML, MP3 e outros formatos sem engine real | ZIP |
| ZIP | nenhum destino — extração ainda não implementada |

- A conversão de **DOCX/PDF** é textual: o conteúdo é extraído, mas layout, imagens, colunas e formatação complexa não são preservados. PDFs digitalizados precisam de OCR, que não está incluído.
- A conversão de **vídeo** usa o FFmpeg instalado no sistema. O app procura o executável em `SHIFTCONVERT_FFMPEG`, no `PATH` e em instalações comuns do WinGet. Sem FFmpeg, arquivos de vídeo oferecem apenas ZIP.
- A roda mostra somente os destinos disponíveis para o arquivo selecionado. ZIP é sempre uma opção para arquivos que não sejam ZIP.

### Como funciona

O ShiftConvert combina uma camada nativa em Rust com uma interface React executada pelo WebView2/Tauri.

- Um watcher verifica `Shift`, botão esquerdo e movimento no Explorer.
- Uma janela transparente, sempre no topo, é posicionada sobre a área de trabalho e registra um **`IDropTarget` OLE**.
- O `CF_HDROP` recebido é lido e o retorno é `DROPEFFECT_COPY`, impedindo que o Explorer mova ou apague o original.
- A roda é desenhada em SVG; o hit-test usa a mesma geometria no lado Rust.
- A saída é escrita em um arquivo temporário e renomeada ao final. Em caso de cancelamento ou erro, o temporário é removido.

```text
src-tauri/src/
  drag.rs          watcher Shift+drag, drop target OLE, picker e HUD
  screen.rs        área de trabalho e DPI por monitor
  overlay.rs       posicionamento e exibição da janela nativa
  conversion.rs    validação, nomes únicos e finalização atômica
  file_info.rs     metadados e lista real de destinos
  engines/
    image.rs       imagens
    text.rs        TXT, Markdown e HTML
    document.rs    DOCX e PDF
    video.rs       vídeo e GIF via FFmpeg
    compression.rs ZIP

src/
  App.tsx
  components/FormatWheel.tsx
  components/StartupPrompt.tsx
  components/StatusHud.tsx
  styles/orbit.css
```

### Desenvolvimento

As instruções abaixo são destinadas a quem vai executar o projeto localmente em modo de desenvolvimento. Usuários finais devem usar o pacote publicado em Releases.

#### Requisitos

- Windows 10/11 x64
- WebView2 Runtime (incluído no Windows 11 e em instalações atualizadas do Edge)
- Node.js 20+ e npm
- Rust stable com toolchain MSVC
- Visual Studio Build Tools com “Desktop development with C++” e `link.exe`
- FFmpeg opcional, para conversões de vídeo: `winget install Gyan.FFmpeg`

#### Executar em desenvolvimento

```powershell
git clone https://github.com/gabriel-bastarz/shiftconvert.git
cd shiftconvert
npm install
npm run tauri dev
```

O comando inicia o Vite em `http://127.0.0.1:5173` e abre o aplicativo Tauri em modo debug. Alterações em arquivos React/CSS são aplicadas por HMR; alterações em Rust exigem recompilação e reinício.

Para ver logs detalhados:

```powershell
$env:RUST_LOG = "debug"
npm run tauri dev
```

#### Testes e verificação

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
npx tsc --noEmit
npm run build
```

### Limitações conhecidas

- Aplicativo para **Windows** e dependente do Windows Explorer.
- Documentos são convertidos apenas como texto; não há OCR nem preservação de layout.
- Vídeo requer FFmpeg; sem ele, a roda oferece somente ZIP para esses arquivos.
- Não há extração de ZIP, conversão de áudio ou preservação de metadados complexos.
- A versão atual não possui instalador, atualização automática ou assinatura de código.
- A detecção do gesto depende das classes de janela do Explorer; outros gerenciadores de arquivos não disparam a roda.
- O projeto está em estágio alpha; mudanças de comportamento e formato ainda são possíveis.

### Privacidade e integridade

- Nenhum arquivo é enviado para a internet.
- Não há telemetria, analytics ou coleta de dados.
- O original é aberto somente para leitura.
- Conversões são finalizadas com arquivo temporário + renomeação.
- Processos externos recebem argumentos separados, sem interpolação de shell.

---

## English

ShiftConvert turns a Windows Explorer drag gesture into a conversion interface. Hold **Shift**, drag a file, and a radial menu appears at the cursor. Hover over a target format and release to convert.

There is no main window and no upload: everything runs locally. The source file is opened read-only and is never moved, modified, or deleted.

### Highlights

- **Shift + drag in Explorer** opens the format wheel at the cursor.
- **Only real conversions** are shown; every option is backed by a supported conversion.
- **The source is preserved** — the drop is handled as a copy and the output is written next to the original.
- **No silent overwrites** — existing names receive ` (1)`, ` (2)`, and so on.
- **Orbit Minimal interface** — opaque graphite surfaces, rounded petals, and electric-blue highlights.
- **DPI and multi-monitor aware** — placement and scale follow the monitor under the cursor.
- **Fully offline** — no telemetry, uploads, or external services.
- **Alternative access** — tray icon and a native file picker.
- **Launch with Windows** — option in the tray menu with a checked/unchecked state.
- **Optional FFmpeg** — enables video conversions when available.
- **Safe cancellation** — temporary files are removed and the source remains untouched.

### How to use

1. Download the latest package from [Releases](https://github.com/gabriel-bastarz/shiftconvert/releases/latest).
2. Extract the ZIP to a folder of your choice and run `ShiftConvert.exe`.
3. In Explorer, hold **Shift**, drag a file, and hover over the target format.
4. Release over the option to convert. The result is saved next to the source file.

The app lives in the Windows system tray. Right-click the icon to open the interface, pick a file, or enable **Launch with Windows**.

> This version is a portable executable: there is no installer or code signing. Windows SmartScreen may warn on first launch. If the executable is moved after enabling launch at startup, re-enable the option in the tray menu.

### Supported formats

| Source | Offered targets |
|---|---|
| JPG/JPEG, PNG, GIF, BMP, WEBP, TIFF/TIF, ICO | other image formats + ZIP |
| TXT | MD, HTML + ZIP |
| MD/Markdown | TXT, HTML + ZIP |
| HTML/HTM | TXT + ZIP |
| MP4, MKV, WEBM, AVI, MOV, WMV, FLV, M4V, MPG/MPEG | MP4, WEBM, MKV, animated GIF (except the same format) + ZIP |
| DOCX | PDF, HTML, TXT + ZIP |
| PDF | DOCX, HTML, TXT + ZIP |
| DOC, ODT, RTF, CSV, JSON, XML, MP3, and other formats without a real engine | ZIP |
| ZIP | no targets — extraction is not implemented yet |

- **DOCX/PDF** conversion is text-only: content is extracted, but layout, images, columns, and complex formatting are not preserved. Scanned PDFs require OCR, which is not included.
- **Video** conversion uses the system FFmpeg. The app looks for it in `SHIFTCONVERT_FFMPEG`, on `PATH`, and in common WinGet install locations. Without FFmpeg, video files only offer ZIP.
- The wheel only shows targets available for the selected file. ZIP is always available for non-ZIP files.

### How it works

ShiftConvert combines a native Rust layer with a React interface running in WebView2/Tauri.

- A watcher checks for `Shift`, the left mouse button, and movement in Explorer.
- A transparent always-on-top window covers the work area and registers an **OLE `IDropTarget`**.
- The received `CF_HDROP` is read and the return value is `DROPEFFECT_COPY`, preventing Explorer from moving or deleting the source.
- The wheel is rendered in SVG; hit-testing uses the same geometry in Rust.
- Output is written to a temporary file and renamed when complete. On cancellation or error, the temporary file is removed.

```text
src-tauri/src/
  drag.rs          Shift+drag watcher, OLE drop target, picker, and HUD
  screen.rs        work area and per-monitor DPI
  overlay.rs       native window placement and display
  conversion.rs    validation, unique names, and atomic finalization
  file_info.rs     metadata and actual target list
  engines/
    image.rs       images
    text.rs        TXT, Markdown, and HTML
    document.rs    DOCX and PDF
    video.rs       video and GIF via FFmpeg
    compression.rs ZIP

src/
  App.tsx
  components/FormatWheel.tsx
  components/StartupPrompt.tsx
  components/StatusHud.tsx
  styles/orbit.css
```

### Development

The instructions below are only for running the project locally in development mode. End users should use the package published in Releases.

#### Requirements

- Windows 10/11 x64
- WebView2 Runtime (included with Windows 11 and up-to-date Edge installations)
- Node.js 20+ and npm
- Rust stable with the MSVC toolchain
- Visual Studio Build Tools with “Desktop development with C++” and `link.exe`
- Optional FFmpeg for video conversions: `winget install Gyan.FFmpeg`

#### Run in development

```powershell
git clone https://github.com/gabriel-bastarz/shiftconvert.git
cd shiftconvert
npm install
npm run tauri dev
```

This starts Vite at `http://127.0.0.1:5173` and opens the Tauri app in debug mode. React/CSS changes are applied through HMR; Rust changes require a rebuild and restart.

For detailed logs:

```powershell
$env:RUST_LOG = "debug"
npm run tauri dev
```

#### Tests and checks

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
npx tsc --noEmit
npm run build
```

### Known limitations

- **Windows-only** application, dependent on Windows Explorer.
- Documents are converted as text only; there is no OCR or layout preservation.
- Video requires FFmpeg; without it, those files only offer ZIP.
- No ZIP extraction, audio conversion, or complex metadata preservation.
- This version has no installer, auto-update, or code signing.
- Drag detection depends on Explorer window classes; other file managers do not trigger the wheel.
- The project is in alpha; behavior and formats may still change.

### Privacy and integrity

- No file is sent over the internet.
- There is no telemetry, analytics, or data collection.
- The source is opened read-only.
- Conversions are finalized with a temporary file and a rename.
- External processes receive separate arguments without shell interpolation.
