param(
    [string]$SourcePath = ""
)

Add-Type -AssemblyName System.Drawing

$baseDir = Split-Path -Parent $PSScriptRoot
$publicDir = Join-Path $baseDir "public\ax-portal"
$nativeDir = Join-Path $baseDir "src-tauri\icons\ax-portal"
$repoSourcePath = Join-Path $nativeDir "icon-source.png"

# Resolve source file:
# 1. Explicit parameter if passed
# 2. Repo master asset (src-tauri\icons\ax-portal\icon-source.png)
# 3. Local machine brain cache if bootstrapping for the first time
if ([string]::IsNullOrWhiteSpace($SourcePath)) {
    if (Test-Path $repoSourcePath) {
        $SourcePath = $repoSourcePath
    } else {
        $brainPath = "C:\Users\Harry\.gemini\antigravity-ide\brain\7c1a6c87-5b84-44e0-838f-03523de3b2c3\ax_portal_app_icon_1792429257568.jpg"
        if (Test-Path $brainPath) {
            $SourcePath = $brainPath
        } else {
            throw "Source icon not found. Please provide -SourcePath <path to source image> or ensure $repoSourcePath exists."
        }
    }
} elseif (-not (Test-Path $SourcePath)) {
    throw "Specified SourcePath does not exist: $SourcePath"
}

Write-Host "Using source image: $SourcePath"

New-Item -ItemType Directory -Force -Path $publicDir | Out-Null
New-Item -ItemType Directory -Force -Path $nativeDir | Out-Null

$rawImg = [System.Drawing.Image]::FromFile($SourcePath)
$rawBmp = New-Object System.Drawing.Bitmap $rawImg

# Check if the image needs squircle alpha masking (e.g. corner pixel (0,0) is opaque)
$cornerPixel = $rawBmp.GetPixel(0, 0)
$needsMask = ($cornerPixel.A -eq 255)

if ($needsMask) {
    Write-Host "Source image is opaque at corners (A=$($cornerPixel.A)). Applying squircle alpha mask..."
    $masterBmp = New-Object System.Drawing.Bitmap(1024, 1024, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $masterBmp.SetResolution($rawImg.HorizontalResolution, $rawImg.VerticalResolution)

    $g = [System.Drawing.Graphics]::FromImage($masterBmp)
    $g.Clear([System.Drawing.Color]::Transparent)
    $g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.CompositingQuality = [System.Drawing.Drawing2D.CompositingQuality]::HighQuality
    $g.InterpolationMode = [System.Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.PixelOffsetMode = [System.Drawing.Drawing2D.PixelOffsetMode]::HighQuality

    $rect = New-Object System.Drawing.RectangleF(128, 127.5, 768, 768)
    $radius = 168.0
    $d = $radius * 2

    $path = New-Object System.Drawing.Drawing2D.GraphicsPath
    $path.AddArc($rect.X, $rect.Y, $d, $d, 180, 90)
    $path.AddArc($rect.Right - $d, $rect.Y, $d, $d, 270, 90)
    $path.AddArc($rect.Right - $d, $rect.Bottom - $d, $d, $d, 0, 90)
    $path.AddArc($rect.X, $rect.Bottom - $d, $d, $d, 90, 90)
    $path.CloseFigure()

    $g.SetClip($path)
    $g.DrawImage($rawImg, 0, 0, 1024, 1024)
    $g.Dispose()

    $masterBmp.Save($repoSourcePath, [System.Drawing.Imaging.ImageFormat]::Png)
    Write-Host "Saved masked master asset to: $repoSourcePath"
} else {
    Write-Host "Source image already has alpha channel at corners (A=$($cornerPixel.A))."
    $masterBmp = $rawBmp
    if ($SourcePath -ne $repoSourcePath) {
        $masterBmp.Save($repoSourcePath, [System.Drawing.Imaging.ImageFormat]::Png)
        Write-Host "Copied master asset to: $repoSourcePath"
    }
}

function Resize-Image($image, $width, $height) {
    $destRect = New-Object System.Drawing.Rectangle(0, 0, $width, $height)
    $destImage = New-Object System.Drawing.Bitmap($width, $height, [System.Drawing.Imaging.PixelFormat]::Format32bppArgb)
    $destImage.SetResolution($image.HorizontalResolution, $image.VerticalResolution)

    $graphics = [System.Drawing.Graphics]::FromImage($destImage)
    $graphics.Clear([System.Drawing.Color]::Transparent)
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
Save-Png $masterBmp 512 512 (Join-Path $publicDir "logo.png")
Save-Png $masterBmp 256 256 (Join-Path $publicDir "icon.png")
Save-Png $masterBmp 32 32 (Join-Path $publicDir "favicon.png")

# 2. Native app icons
Save-Png $masterBmp 512 512 (Join-Path $nativeDir "icon.png")
Save-Png $masterBmp 256 256 (Join-Path $nativeDir "128x128@2x.png")
Save-Png $masterBmp 128 128 (Join-Path $nativeDir "128x128.png")
Save-Png $masterBmp 64 64 (Join-Path $nativeDir "64x64.png")
Save-Png $masterBmp 32 32 (Join-Path $nativeDir "32x32.png")

# 3. Native multi-resolution icon.ico
$icoSizes = @(16, 24, 32, 48, 64, 128, 256)
$icoPngBytes = @()

foreach ($sz in $icoSizes) {
    $resized = Resize-Image $masterBmp $sz $sz
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

if ($masterBmp -ne $rawBmp) {
    $masterBmp.Dispose()
}
$rawBmp.Dispose()
$rawImg.Dispose()

Write-Host "Saved multi-resolution ICO: $icoPath"
