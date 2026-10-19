Add-Type -AssemblyName System.Drawing

$sourcePath = "C:\Users\Harry\.gemini\antigravity-ide\brain\7c1a6c87-5b84-44e0-838f-03523de3b2c3\ax_portal_app_icon_1792429257568.jpg"
$baseDir = Split-Path -Parent $PSScriptRoot
$publicDir = Join-Path $baseDir "public\ax-portal"
$nativeDir = Join-Path $baseDir "src-tauri\icons\ax-portal"

New-Item -ItemType Directory -Force -Path $publicDir | Out-Null
New-Item -ItemType Directory -Force -Path $nativeDir | Out-Null

$srcBmp = [System.Drawing.Image]::FromFile($sourcePath)

function Resize-Image($image, $width, $height) {
    $destRect = New-Object System.Drawing.Rectangle(0, 0, $width, $height)
    $destImage = New-Object System.Drawing.Bitmap($width, $height)
    $destImage.SetResolution($image.HorizontalResolution, $image.VerticalResolution)

    $graphics = [System.Drawing.Graphics]::FromImage($destImage)
    $graphics.CompositingMode = [System.Drawing.Drawing2D.CompositingMode]::SourceCopy
    $graphics.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $graphics.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $graphics.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::HighQuality
    $graphics.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality

    $wrapMode = New-Object System.Drawing.Imaging.ImageAttributes
    $wrapMode.SetWrapMode([System.Drawing.Drawing2D.WrapMode]::TileFlipXY)
    $graphics.DrawImage($image, $destRect, 0, 0, $image.Width, $image.Height, [System.Drawing.GraphicsUnit]::Pixel, $wrapMode)
    $graphics.Dispose()
    return $destImage
}

function Save-Png($image, $width, $height, $outPath) {
    $resized = Resize-Image $image $width $height
    $resized.Save($outPath, [System.Drawing.Imaging.ImageFormat]::Png)
    $resized.Dispose()
    Write-Host "Saved: $outPath ($width x $height)"
}

# 1. Public web assets
Save-Png $srcBmp 512 512 (Join-Path $publicDir "logo.png")
Save-Png $srcBmp 256 256 (Join-Path $publicDir "icon.png")
Save-Png $srcBmp 32 32 (Join-Path $publicDir "favicon.png")

# 2. Native app icons
Save-Png $srcBmp 512 512 (Join-Path $nativeDir "icon.png")
Save-Png $srcBmp 256 256 (Join-Path $nativeDir "128x128@2x.png")
Save-Png $srcBmp 128 128 (Join-Path $nativeDir "128x128.png")
Save-Png $srcBmp 64 64 (Join-Path $nativeDir "64x64.png")
Save-Png $srcBmp 32 32 (Join-Path $nativeDir "32x32.png")

# 3. Native multi-resolution icon.ico
$icoSizes = @(16, 24, 32, 48, 64, 128, 256)
$icoPngBytes = @()

foreach ($sz in $icoSizes) {
    $resized = Resize-Image $srcBmp $sz $sz
    $ms = New-Object System.IO.MemoryStream
    $resized.Save($ms, [System.Drawing.Imaging.ImageFormat]::Png)
    $icoPngBytes += ,@($sz, $ms.ToArray())
    $ms.Dispose()
    $resized.Dispose()
}

$icoPath = Join-Path $nativeDir "icon.ico"
$fs = [System.IO.File]::Create($icoPath)
$bw = New-Object System.IO.BinaryWriter($fs)

# ICONDIR header
$bw.Write([uint16]0) # Reserved
$bw.Write([uint16]1) # Type 1 = ICO
$bw.Write([uint16]$icoSizes.Count) # Count

# Header size: 6 + 16 * count
$offset = 6 + 16 * $icoSizes.Count

foreach ($item in $icoPngBytes) {
    $sz = $item[0]
    $bytes = $item[1]
    
    $w = if ($sz -ge 256) { 0 } else { [byte]$sz }
    $h = if ($sz -ge 256) { 0 } else { [byte]$sz }
    
    $bw.Write([byte]$w)
    $bw.Write([byte]$h)
    $bw.Write([byte]0) # Palette count
    $bw.Write([byte]0) # Reserved
    $bw.Write([uint16]1) # Color planes
    $bw.Write([uint16]32) # Bits per pixel
    $bw.Write([uint32]$bytes.Length) # Image size
    $bw.Write([uint32]$offset) # Offset
    
    $offset += $bytes.Length
}

foreach ($item in $icoPngBytes) {
    $bw.Write($item[1])
}

$bw.Flush()
$bw.Close()
$fs.Close()

$srcBmp.Dispose()
Write-Host "Saved multi-resolution ICO: $icoPath"
