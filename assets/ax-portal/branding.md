# AX Portal Branding & Asset Documentation

## Overview
**AX Portal** is the internal desktop portal for enterprise AI skills management, discovery, and installation. This document records the brand assets, design prompts, tools, and technical specifications for the visual identity and native desktop application icons.

---

## 1. Design Concept & Philosophy
- **Name**: AX Portal
- **Visual Motif**: A geometric dimensional monogram of the letters **A** and **X** framing an inner luminous circular portal gateway.
- **Color Palette**:
  - Primary Enterprise Blue: `#1657b8`
  - Soft Highlight / Cyan Glow: `#8ab8fa` / `#73d9b7`
  - Base Dark Surface: `#16191e` / `#20242b`
  - Contrast Light Surface: `#ffffff`
- **Aesthetic**: Minimalist, clean-edged, high-tech modern enterprise software aesthetic on a subtle dark rounded squircle tile.

---

## 2. Master Asset Generation
- **Generation Engine**: Google DeepMind Antigravity Image Generation Tool (`generate_image`)
- **Model Resolution**: 1024 × 1024 px
- **Prompt**:
  ```text
  Professional modern app icon for AX Portal desktop application, featuring a sleek geometric monogram of the letters AX forming an elegant dimensional portal gateway, vibrant enterprise blue, indigo, and cyan glowing accents on a smooth dark slate rounded squircle background, minimalist, high tech, premium UI design, 3D modern app icon style, vector finish, clean lines, no clutter, 1024x1024
  ```

---

## 3. Asset Specifications & Manifest

### Web & In-App UI Assets (`public/ax-portal/`)
| File | Dimensions | Purpose |
| :--- | :--- | :--- |
| `logo.png` | 512 × 512 | High-resolution branding display |
| `icon.png` | 256 × 256 | Sidebar brand icon and dialogs |
| `favicon.png` | 32 × 32 | Web browser tab icon & preview favicon |

### Native Desktop & Bundle Icons (`src-tauri/icons/ax-portal/`)
| File | Dimensions | Purpose |
| :--- | :--- | :--- |
| `32x32.png` | 32 × 32 | System tray / low-DPI taskbar icon |
| `64x64.png` | 64 × 64 | Medium density native display |
| `128x128.png` | 128 × 128 | High-DPI application icon |
| `128x128@2x.png` | 256 × 256 | Retina / 200% scale display icon |
| `icon.png` | 512 × 512 | Windows Store & installer banner master |
| `icon.ico` | Multi-res | Standard Windows binary icon (16, 24, 32, 48, 64, 128, 256 px) |

---

## 4. Automation & Maintenance
All derived PNGs and the Windows `.ico` container are generated programmatically using `scripts/generate-ax-icons.ps1` using `System.Drawing` with `HighQualityBicubic` interpolation and tile flip wrap mode.

To regenerate icons from a new master image, execute:
```powershell
powershell -ExecutionPolicy Bypass -File scripts/generate-ax-icons.ps1
```
