[CmdletBinding()]
param(
    [switch]$Force
)

$ErrorActionPreference = "Stop"

$version = "9.0.2"
$archiveName = "ffmpeg-$version-essentials_build.7z"
$archiveUrl = "https://www.gyan.dev/ffmpeg/builds/packages/$archiveName"
$archiveSha256 = "4705843CCAAF54257C16AD90F3E952ECE33C17DF964ECF7BFDBB0F49C7171077"
$binarySha256 = "3256173F3F8BFFD7DF12227C68ADF68025EDB1832273A9530688A7BB1ED8EDEC"

$projectRoot = Split-Path -Parent $PSScriptRoot
$resourceDir = Join-Path $projectRoot "src-tauri\resources\ffmpeg"
$cacheDir = Join-Path $projectRoot "src-tauri\resources\.cache"
$archivePath = Join-Path $cacheDir $archiveName
$ffmpegPath = Join-Path $resourceDir "ffmpeg.exe"
$licensePath = Join-Path $resourceDir "LICENSE"
$readmePath = Join-Path $resourceDir "README.txt"
$sourcePath = Join-Path $resourceDir "SOURCE.txt"

function Test-FileHash {
    param(
        [Parameter(Mandatory = $true)][string]$Path,
        [Parameter(Mandatory = $true)][string]$Expected
    )

    if (-not (Test-Path -LiteralPath $Path -PathType Leaf)) {
        return $false
    }

    return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash -eq $Expected
}

if (
    -not $Force -and
    (Test-FileHash -Path $ffmpegPath -Expected $binarySha256) -and
    (Test-Path -LiteralPath $licensePath -PathType Leaf) -and
    (Test-Path -LiteralPath $readmePath -PathType Leaf) -and
    (Test-Path -LiteralPath $sourcePath -PathType Leaf)
) {
    Write-Host "FFmpeg $version ja esta preparado em $resourceDir"
    exit 0
}

$tar = Get-Command tar -ErrorAction SilentlyContinue
if (-not $tar) {
    throw "O comando tar nao foi encontrado. Ele e necessario para extrair o pacote 7z do FFmpeg."
}

New-Item -ItemType Directory -Force -Path $resourceDir | Out-Null
New-Item -ItemType Directory -Force -Path $cacheDir | Out-Null

if (-not (Test-FileHash -Path $archivePath -Expected $archiveSha256)) {
    if (Test-Path -LiteralPath $archivePath) {
        Remove-Item -LiteralPath $archivePath -Force
    }

    Write-Host "Baixando FFmpeg $version (essentials, Windows x64)..."
    Invoke-WebRequest -Uri $archiveUrl -OutFile $archivePath -UseBasicParsing
}

if (-not (Test-FileHash -Path $archivePath -Expected $archiveSha256)) {
    throw "O SHA-256 do pacote FFmpeg nao confere. O download foi descartado."
}

$tempDir = Join-Path ([IO.Path]::GetTempPath()) ("shiftconvert-ffmpeg-" + [guid]::NewGuid().ToString("N"))
New-Item -ItemType Directory -Path $tempDir | Out-Null

try {
    $archiveRoot = "ffmpeg-$version-essentials_build"
    & $tar.Source -xf $archivePath -C $tempDir `
        "$archiveRoot/bin/ffmpeg.exe" `
        "$archiveRoot/LICENSE" `
        "$archiveRoot/README.txt"

    if ($LASTEXITCODE -ne 0) {
        throw "Nao foi possivel extrair o pacote FFmpeg."
    }

    $extractedDir = Join-Path $tempDir $archiveRoot
    $extractedFfmpeg = Join-Path $extractedDir "bin\ffmpeg.exe"

    if (-not (Test-FileHash -Path $extractedFfmpeg -Expected $binarySha256)) {
        throw "O SHA-256 do ffmpeg.exe extraido nao confere."
    }

    Copy-Item -LiteralPath $extractedFfmpeg -Destination $ffmpegPath -Force
    Copy-Item -LiteralPath (Join-Path $extractedDir "LICENSE") -Destination $licensePath -Force
    Copy-Item -LiteralPath (Join-Path $extractedDir "README.txt") -Destination $readmePath -Force

    $source = @"
FFmpeg $version essentials build for Windows x64
Copyright (c) 2000-2026 the FFmpeg developers
License: GPLv3 (see LICENSE in this folder)

Binary package:
$archiveUrl
SHA-256: $archiveSha256

ffmpeg.exe SHA-256:
$binarySha256

FFmpeg source code:
https://github.com/FFmpeg/FFmpeg/commit/946fcce07b

Build scripts:
https://github.com/GyanD/codexffmpeg

This binary is distributed alongside ShiftConvert as a separate executable.
"@

    [IO.File]::WriteAllText($sourcePath, $source, [Text.UTF8Encoding]::new($false))
    Write-Host "FFmpeg $version preparado em $resourceDir"
}
finally {
    Remove-Item -LiteralPath $tempDir -Recurse -Force -ErrorAction SilentlyContinue
}
