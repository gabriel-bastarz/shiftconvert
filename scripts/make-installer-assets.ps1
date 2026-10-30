$ErrorActionPreference = "Stop"
Add-Type -AssemblyName System.Drawing

$projectRoot = Split-Path -Parent $PSScriptRoot
$installerDir = Join-Path $projectRoot "src-tauri\installer"
$iconPath = Join-Path $projectRoot "src-tauri\icons\icon.png"

New-Item -ItemType Directory -Force -Path $installerDir | Out-Null

$background = [Drawing.Color]::FromArgb(11, 13, 16)
$backgroundAlt = [Drawing.Color]::FromArgb(18, 24, 32)
$accent = [Drawing.Color]::FromArgb(77, 163, 255)
$text = [Drawing.Color]::FromArgb(242, 246, 250)
$muted = [Drawing.Color]::FromArgb(154, 167, 180)

function New-Brush([Drawing.Color]$color) {
    return New-Object Drawing.SolidBrush($color)
}

$icon = [Drawing.Image]::FromFile($iconPath)

try {
    $header = New-Object Drawing.Bitmap(150, 57, [Drawing.Imaging.PixelFormat]::Format24bppRgb)
    $headerGraphics = [Drawing.Graphics]::FromImage($header)
    $headerGraphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $headerGraphics.TextRenderingHint = [Drawing.Text.TextRenderingHint]::ClearTypeGridFit
    $headerGraphics.FillRectangle((New-Brush $background), 0, 0, 150, 57)
    $headerGraphics.DrawImage($icon, 12, 9, 39, 39)
    $headerGraphics.FillRectangle((New-Brush $accent), 0, 54, 150, 3)
    $headerFont = New-Object Drawing.Font("Segoe UI Semibold", 10.5, [Drawing.FontStyle]::Regular, [Drawing.GraphicsUnit]::Point)
    $headerGraphics.DrawString("ShiftConvert", $headerFont, (New-Brush $text), 58, 17)
    $headerGraphics.Dispose()
    $header.Save((Join-Path $installerDir "header.bmp"), [Drawing.Imaging.ImageFormat]::Bmp)
    $header.Dispose()

    $sidebar = New-Object Drawing.Bitmap(164, 314, [Drawing.Imaging.PixelFormat]::Format24bppRgb)
    $sidebarGraphics = [Drawing.Graphics]::FromImage($sidebar)
    $sidebarGraphics.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $sidebarGraphics.TextRenderingHint = [Drawing.Text.TextRenderingHint]::ClearTypeGridFit
    $gradient = New-Object Drawing.Drawing2D.LinearGradientBrush(
        (New-Object Drawing.Rectangle(0, 0, 164, 314)),
        $background,
        $backgroundAlt,
        [Drawing.Drawing2D.LinearGradientMode]::Vertical
    )
    $sidebarGraphics.FillRectangle($gradient, 0, 0, 164, 314)
    $sidebarGraphics.FillRectangle((New-Brush $accent), 0, 0, 3, 314)
    $sidebarGraphics.DrawImage($icon, 38, 44, 88, 88)

    $titleFont = New-Object Drawing.Font("Segoe UI Semibold", 13, [Drawing.FontStyle]::Regular, [Drawing.GraphicsUnit]::Point)
    $bodyFont = New-Object Drawing.Font("Segoe UI", 8.5, [Drawing.FontStyle]::Regular, [Drawing.GraphicsUnit]::Point)
    $accentFont = New-Object Drawing.Font("Segoe UI Semibold", 8.5, [Drawing.FontStyle]::Regular, [Drawing.GraphicsUnit]::Point)
    $center = New-Object Drawing.StringFormat
    $center.Alignment = [Drawing.StringAlignment]::Center
    $center.LineAlignment = [Drawing.StringAlignment]::Center

    $sidebarGraphics.DrawString("ShiftConvert", $titleFont, (New-Brush $text), (New-Object Drawing.RectangleF(0, 146, 164, 30)), $center)
    $sidebarGraphics.DrawString("Shift + arraste", $accentFont, (New-Brush $accent), (New-Object Drawing.RectangleF(0, 180, 164, 24)), $center)
    $sidebarGraphics.DrawString("Conversor de arquivos local", $bodyFont, (New-Brush $muted), (New-Object Drawing.RectangleF(10, 208, 144, 24)), $center)
    $sidebarGraphics.FillRectangle((New-Brush $accent), 0, 311, 164, 3)
    $sidebarGraphics.Dispose()
    $sidebar.Save((Join-Path $installerDir "sidebar.bmp"), [Drawing.Imaging.ImageFormat]::Bmp)
    $sidebar.Dispose()
}
finally {
    $icon.Dispose()
}

Write-Host "Imagens do instalador geradas em $installerDir"
