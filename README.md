# ShiftConvert

Conversor de arquivos acionado por **Shift + arrastar** no Windows Explorer, com uma roda radial **Orbit Minimal** — superfícies opacas em grafite, pétalas arredondadas e destaque azul apenas na opção ativa — que aparece durante o próprio arraste.

O arquivo original **nunca é movido, alterado ou apagado**. A conversão é salva ao lado da origem, com nome livre (sem sobrescrita silenciosa).

---

## O que funciona hoje

- **Shift + botão esquerdo arrastando** um arquivo do Explorer → a roda de formatos abre no cursor.
- **Soltar sobre uma região** converte para aquele formato; **hover apenas destaca** (nunca converte).
- **Somente conversões reais** aparecem na roda:
  - Imagens (`jpg`, `png`, `gif`, `bmp`, `webp`, `tiff`, `ico`): os demais formatos de imagem + `ZIP`.
  - Texto: `TXT` → `MD`/`HTML`; `MD` → `TXT`/`HTML`; `HTML` → `TXT`; sempre + `ZIP`.
  - Vídeo (`mp4`, `mkv`, `webm`, `avi`, `mov`, `wmv`, `flv`, `m4v`, `mpg`, `mpeg`): `MP4`, `WEBM`, `MKV`, `GIF` (exceto o mesmo formato) + `ZIP`, via FFmpeg.
  - PDF, DOCX, áudio e outros formatos sem engine real: apenas `ZIP`.
  - Arquivos `ZIP` não oferecem formatos (extração não implementada).
- **Salva ao lado da origem** (`foto.png` → `foto.jpg`, `foto (1).jpg` se já existir).
- **Progresso discreto** em um HUD com “Abrir pasta” e “Fechar”; erros aparecem sem interromper o app.
- **Esc durante o arraste** cancela a roda, mas mantém o escudo até soltar o botão (o Explorer não move o arquivo).
- **Fallback**: ícone na bandeja e botão “Escolher arquivo” (seletor nativo).
- **Multi-monitor / DPI misto**: posicionamento e escala usam o monitor onde está o cursor.
- Roda adaptativa: distribui de 1 a 6 pétalas e cresce para 7+ quando necessário, sem categorias e sem ícones — apenas rótulos diretos (`PNG`, `TXT`, `MP4`, `ZIP`…).

### Formatos suportados

| Origem | Destinos oferecidos |
|---|---|
| JPG/JPEG, PNG, GIF, BMP, WEBP, TIFF/TIF, ICO | demais formatos de imagem + ZIP |
| TXT | MD, HTML + ZIP |
| MD/Markdown | TXT, HTML + ZIP |
| HTML/HTM | TXT + ZIP |
| MP4, MKV, WEBM, AVI, MOV, WMV, FLV, M4V, MPG/MPEG | MP4, WEBM, MKV, GIF (exceto o mesmo formato) + ZIP |
| PDF, DOCX, CSV, JSON, XML, MP3, etc. | ZIP |
| ZIP | nenhum (extração não implementada) |

> Áudio e documentos binários **não** são simulados. O vídeo usa o FFmpeg instalado no sistema; os destinos de vídeo só aparecem na roda quando o FFmpeg está disponível.

---

## Como funciona (arquitetura)

```
src-tauri/src/
├── drag.rs          # watcher Shift+drag, escudo OLE IDropTarget, geometria/hit-test, picker, HUD
├── screen.rs        # work area e DPI por monitor
├── overlay.rs       # posicionamento Win32, remoção da moldura, show/hide sem ativar
├── conversion.rs    # validação, nome único, temporário + rename, cancelamento, testes
├── file_info.rs     # metadados e lista real de destinos
└── engines/
    ├── image.rs        # conversão real de imagens (crate image)
    ├── text.rs         # TXT/Markdown/HTML (pulldown-cmark + html2text)
    ├── video.rs        # MP4/WEBM/MKV/GIF via FFmpeg, cancelável
    └── compression.rs  # ZIP real e cancelável
src/
├── App.tsx          # máquina de estados + eventos Tauri
├── components/
│   ├── FormatWheel.tsx   # SVG: pétalas Orbit Minimal, núcleo, rótulos, áreas clicáveis
│   ├── StartupPrompt.tsx # cartão inicial
│   └── StatusHud.tsx     # progresso/erro
└── styles/orbit.css
```

- **Captura do arraste**: um watcher consulta `GetAsyncKeyState` a cada 22 ms, exige Shift + botão esquerdo + movimento (> 8 px) e confirma que o arraste começou em uma janela do Explorer. Uma janela-escudo transparente, sempre no topo, registra um **`IDropTarget` OLE**; o `CF_HDROP` entregue é lido e o retorno é `DROPEFFECT_COPY`, então o Explorer nunca move/apaga a origem.
- **Roda**: uma única janela Tauri transparente é reposicionada para cobrir a área de trabalho; o frontend desenha a roda em SVG e o Rust faz o hit-test com a mesma geometria.
- **Orbit Minimal**: o disco é opaco em grafite; as pétalas são geradas por uma malha radial com costuras curvas compartilhadas, suavização Chaikin nos cantos e recortes de aro/núcleo. O azul elétrico é reservado ao hover, com preenchimento, contorno e brilho discretos. Não há refração nem blur do desktop.
- **Salvamento**: escrita em arquivo temporário na pasta de destino e `rename` para o nome final; cancelamento remove o temporário e nunca toca na origem.

---

## Como executar

### Pré-requisitos

- Windows 10/11 com WebView2 (já vem no Windows 11 / Edge atualizado)
- Node.js 18+
- Rust (target MSVC) + Visual C++ Build Tools (`link.exe`)
- FFmpeg no `PATH` para vídeo: `winget install Gyan.FFmpeg` (ou defina `SHIFTCONVERT_FFMPEG` com o caminho do `ffmpeg.exe`)

### Desenvolvimento

O app em dev usa o Vite em `http://127.0.0.1:5173` (fixo em `tauri.conf.json`). Sem o Vite, a janela abre vazia.

```powershell
npm install
npm run dev -- --host 127.0.0.1
```

Em outro terminal:

```powershell
$env:PATH = "$HOME\.cargo\bin;" + $env:PATH
$env:RUST_LOG = "debug"
.\src-tauri\target\debug\shiftconvert.exe
```

Alterações em CSS/TSX são aplicadas pelo HMR; mudanças em Rust exigem `cargo build` e reiniciar o app.

### Release

```powershell
npm run tauri build
.\src-tauri\target\release\shiftconvert.exe
```

`bundle.active` está `false`: o resultado é um `.exe` isolado (sem instalador), que carrega o frontend de `dist/` embutido. Use `npm run tauri build` (e não apenas `cargo build --release`) para que o `dist/` mais recente seja embutido.

### Testes

```powershell
Set-Location src-tauri
cargo test
npx tsc --noEmit    # raiz do projeto
npm run build
```

---

## Ajustar a aparência

| O que mudar | Onde |
|---|---|
| Cores, sombras, fonte e tema do cartão/HUD | `src/styles/orbit.css` (`:root` e classes) |
| Tamanho/raio do cartão inicial | `tauri.conf.json` (`width`/`height`) + `overlay::show_prompt` |
| Tamanho do HUD | `drag::show_hud_at` (420×204 CSS) — mantenha o CSS do `.hud-card` compatível |
| Diâmetro/raios/curvatura da roda | `drag.rs` → `wheel_target_physical`, `make_layout` (`outer`, `inner`, `seam_bends`) |
| Forma das pétalas, vãos, aro e núcleo | `FormatWheel.tsx` → constantes `rimGap`, `hubGap`, `panelGap` e `petalPath` |
| Tamanho/posição dos rótulos | `FormatWheel.tsx` → bloco `geometry.labels` (`radius`, `fontSize`, `letterSpacing`) |
| Fundo grafite, painéis e hover azul | gradientes `orbit-disc`, `orbit-panel`, `orbit-hover`, `orbit-hub` |
| Animação de entrada e hover | `orbit.css` → `.wheel-group`, `.orbit-hover-path` |

O hit-test do Rust e as áreas clicáveis do SVG precisam continuar iguais: ambos usam os mesmos raios e ângulos (`seamAngle`, `outerRadiusAt`, `innerRadiusAt`).

---

## Segurança e integridade

- Nenhum dado sai para a internet; não há telemetria.
- O original é aberto apenas para leitura.
- Sem sobrescrita: colisões viram `nome (1).ext`.
- Caminhos validados antes de qualquer operação.
- Processos externos usam argumentos separados (`explorer.exe /select,<caminho>`), sem concatenação de shell.

---

## Limitações conhecidas

- A roda é **opaca**: não usa transparência, refração ou blur do desktop atrás dela; apenas a janela ao redor do disco é transparente.
- Texto limitado a TXT, Markdown e HTML; PDF, DOCX, ODT e planilhas ainda não têm engine.
- Vídeo exige FFmpeg; sem ele a roda oferece apenas ZIP para arquivos de vídeo.
- **Sem extração de ZIP** e sem conversão de áudio.
- GIF de saída é sempre a imagem estática do primeiro quadro; GIF de vídeo é animado.
- A detecção depende das classes de janela do Explorer; outros gerenciadores de arquivos não disparam a roda.
- Sem instalador, auto-update ou assinatura de código.
- Em monitores com escalas diferentes, a roda usa a escala do monitor do cursor no momento do gesto.

## Solução de problemas

- **Cartão não aparece em dev**: confirme `npm run dev -- --host 127.0.0.1` e que o app foi iniciado depois do Vite.
- **Janela antiga presa / tela estranha**: feche `shiftconvert.exe` e eventuais `msedgewebview2.exe` órfãos e reabra.
- **Shift + arrastar não detecta**: inicie o arraste dentro de uma janela do Explorer e mova mais de 8 px com Shift pressionado.
- **Destinos de vídeo não aparecem**: confirme `ffmpeg -version` no `PATH` (ou defina `SHIFTCONVERT_FFMPEG`) e reabra o ShiftConvert.
- **Logs**: rode com `$env:RUST_LOG = "debug"`.
