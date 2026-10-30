# FFmpeg para o instalador

Esta pasta é preenchida por:

```powershell
npm run fetch:ffmpeg
```

O script baixa o build **FFmpeg 9.0.2 essentials para Windows x64** de
`gyan.dev`, confere o SHA-256 do pacote e do `ffmpeg.exe` e copia para cá:

- `ffmpeg.exe`
- `LICENSE`
- `README.txt`
- `SOURCE.txt`

O `ffmpeg.exe` é um binário separado, licenciado sob GPLv3. Consulte
`THIRD_PARTY_NOTICES.md` na raiz do repositório.

Os arquivos baixados não são versionados. O cache do pacote 7z fica em
`src-tauri/resources/.cache/`.
