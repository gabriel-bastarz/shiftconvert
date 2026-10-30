; Limpa a entrada de inicializacao automatica do ShiftConvert antes de remover
; os arquivos. A entrada fica no registro do usuario atual.
!macro NSIS_HOOK_PREUNINSTALL
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Run" "ShiftConvert"
  DeleteRegValue HKCU "Software\Microsoft\Windows\CurrentVersion\Explorer\StartupApproved\Run" "ShiftConvert"
  ; Remove o diretorio lembrado pelo instalador NSIS.
  DeleteRegKey HKCU "Software\Gabriel Bastarz\ShiftConvert"
  DeleteRegKey /ifempty HKCU "Software\Gabriel Bastarz"
  DeleteRegKey HKCU "Software\shiftconvert\ShiftConvert"
  DeleteRegKey /ifempty HKCU "Software\shiftconvert"
!macroend
