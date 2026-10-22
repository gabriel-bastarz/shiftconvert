# Análise Técnica: Detecção de Shift + Arrastar no Windows

> **Atualização — implementado:** a limitação descrita abaixo foi resolvida. O app hoje usa a alternativa A (janela-escudo transparente e sempre no topo registrada como `IDropTarget` OLE), com uma diferença decisiva: o escudo é exibido assim que o watcher detecta Shift + arrasto, não apenas no momento do drop. Assim a roda aparece durante o gesto. O `CF_HDROP` é lido e o alvo retorna `DROPEFFECT_COPY`, então o Explorer nunca move nem apaga a origem. Este documento permanece como registro histórico da investigação.

## Resumo

O acionamento desejado — **Shift + arrastar com botão esquerdo** originado no Windows Explorer — **não é viável de forma confiável** com a arquitetura inicial de um aplicativo desktop comum (Tauri + hook global de teclado/mouse).

Este documento explica por quê, quais alternativas foram investigadas e qual abordagem foi implementada como prova de conceito.

---

## 1. O problema

Quando o usuário seleciona um arquivo no Explorer, segura Shift e começa a arrastar com o botão esquerdo, o Windows:

1. Inicia uma operação de arraste e soltar (drag-and-drop) interna do Explorer.
2. O arquivo arrastado é encapsulado em um `IDataObject` (formato `CF_HDROP` / `FileGroupDescriptor`).
3. Esse `IDataObject` é disponibilizado apenas para **alvos de soltar (drop targets)** registrados, não para processos externos que queiram "espiar" o arraste.

Um hook global de baixo nível (`WH_MOUSE_LL` / `WH_KEYBOARD_LL`) pode detectar:
- Que a tecla Shift está pressionada (`GetAsyncKeyState(VK_SHIFT)`).
- Que o botão esquerdo do mouse foi pressionado e está se movendo.

Porém, **não consegue associar esse evento de mouse a um arquivo específico do Explorer**, porque:
- O Explorer não envia o caminho do arquivo para processos externos durante o início do arraste.
- Não há API pública do Windows que notifique "o usuário começou a arrastar o arquivo X com Shift pressionado".

---

## 2. Alternativas investigadas

| Alternativa | Viável? | Observações |
|-------------|---------|-------------|
| Hook global `WH_MOUSE_LL` + `WH_KEYBOARD_LL` | Parcial | Detecta Shift + mouse, mas não o arquivo arrastado. |
| Janela overlay transparente como drop target | Sim | Requer que o usuário solte o arquivo sobre a overlay. Não detecta o início do arraste. |
| Integração COM com `IDataObject` / `IDropSource` do Explorer | Complexa | Exige injeção ou registro como drop target; não é trivial em Tauri. |
| Menu de contexto do Explorer (Shell Extension) | Sim | Exige registro no registro do Windows (`.dll` COM). Fora do escopo Tauri puro. |
| Atalho global (ex.: Shift + clique direito) | Sim | Abre menu mas não obtém o arquivo automaticamente. |
| Ícone da bandeja + seletor de arquivo | Sim | Fallback confiável, mas menos ágil. |

---

## 3. O que foi implementado (Fase 1 — Prova de Conceito)

### `src-tauri/src/windows_input.rs`

Inicializa uma estrutura `WindowsInputHook` que, em um ambiente de produção, instalaria `SetWindowsHookEx` com `WH_MOUSE_LL` e `WH_KEYBOARD_LL`. No estado atual:

- O hook é inicializado como estrutura de suporte.
- Não captura eventos reais de arraste do Explorer (limitação documentada acima).
- Emite eventos `shiftconvert:activation` para o frontend via Tauri event system.

### `src-tauri/src/main.rs` — Comando `request_activation`

Comando Tauri que permite ativação manual:
- Recebe `filePath`, `cursorX`, `cursorY`.
- Valida que o arquivo existe via `file_info::inspect_file`.
- Posiciona a janela principal perto do cursor.
- Emite o payload de ativação para o frontend.

### Frontend (`src/App.tsx`, `src/components/RadialMenu.tsx`)

- Escuta o evento `shiftconvert:activation`.
- Abre o menu radial na posição do cursor.
- Se não houver arquivo (`filePath` null), exibe um erro e um botão de ativação manual.

---

## 4. Limitações conhecidas

1. **Não há detecção automática de Shift + drag do Explorer.** O arquivo não é obtido automaticamente durante o arraste.
2. **O fallback manual é necessário.** O usuário precisa selecionar o arquivo por outro meio (bandeja, atalho global, ou arrastar para uma janela overlay).
3. **A janela overlay transparente não recebe arquivos arrastados do Explorer** por padrão, pois o Explorer não solta arquivos em janelas que não são drop targets registrados.

---

## 5. Alternativa técnica recomendada

Para alcançar a experiência exata (Shift + arrastar do Explorer abre o menu com o arquivo correto), recomenda-se uma das seguintes abordagens, em ordem de complexidade:

### A. Overlay transparente como drop target (recomendado para MVP)

1. Cria uma janela overlay transparente, sempre no topo, que cobre a tela ou segue o cursor.
2. Registra essa janela como um **drop target** do Windows (`IDropTarget`, `OleRegisterDropTarget`).
3. Quando o usuário solta o arquivo sobre a overlay, o Windows entrega o `IDataObject` com o(s) caminho(s) do(s) arquivo(s).
4. O aplicativo abre o menu radial com o arquivo identificado.

**Vantagem:** não exige shell extension nem injeção no Explorer.
**Desvantagem:** o usuário precisa **soltar** o arquivo, não apenas começar a arrastar.

### B. Shell Extension (menu de contexto do Explorer)

1. Registra uma extensão de shell no Windows que adiciona uma opção "Converter com ShiftConvert" ao menu de contexto do Explorer.
2. Ao clicar, o Explorer passa o caminho do arquivo para o aplicativo.

**Vantagem:** obtém o arquivo diretamente do Explorer.
**Desvantagem:** exige registro no registro do Windows, compilação de DLL COM, e instalação com privilégios de administrador.

### C. Atalho global + monitoramento de área de transferência

1. O usuário copia o arquivo no Explorer (`Ctrl+C`).
2. Shift + atalho global detecta a combinação e lê a área de transferência (`CF_HDROP`).
3. Abre o menu com o arquivo.

---

## 6. Conclusão para o MVP

A implementação atual valida:
- Inicialização do aplicativo Tauri.
- Janela transparente e posicionamento próximo ao cursor.
- Roda radial Orbit Minimal com pétalas adaptativas.
- Conversões reais: imagens, TXT/Markdown/HTML, vídeo via FFmpeg e ZIP.

A detecção de **Shift + arrastar do Explorer** é feita pelo watcher nativo e o overlay atua como drop target OLE (opção A acima), retornando `DROPEFFECT_COPY` para preservar a origem.
