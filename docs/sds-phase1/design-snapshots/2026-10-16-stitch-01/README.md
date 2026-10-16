# Stitch snapshot 2026-10-16-stitch-01

Read-only capture of project 9226348915970684263 on 2026-10-16 (Asia/Saigon). Project updateTime: 2026-10-16T15:14:10.500363Z. This is a retrieval version, not a screen modification timestamp.

- `project.json`: project metadata, canvas instances and theme.
- `screen-list.json`: complete list_screens response.
- `screens.json`: first get_screen pass (one transient W01 retrieval error).
- `screens-retrieved.json`: complete successful get_screen results, including retried W01.
- `design-systems.json`: available design-system definitions.
- `before/`: immutable local documentation/prototype/preview baseline captured before edits.
- `manifest.json`: downloaded file sizes and SHA-256 checksums.
- `contact-0.png` through `contact-4.png`: overview sheets of all retrieved screenshots.
- `changes.diff`: Git no-index diff of this synchronization, excluding itself.
- [Comparison and review issues](../../STITCH-SYNC-2026-10-16.md).

28 wireframe HTML files and 29 screenshots (14 PNG, 15 JPEG; extensions follow actual file signatures). The reference image has no HTML. HTML exports are preserved verbatim and may load external CDN assets if opened; Saved screenshots are the self-contained visual reference. Neither scripts nor generated command suggestions were executed.

## Screen inventory

The existing primary mapping remains in [STITCH-TRANSFER](../../STITCH-TRANSFER.md). Duplicate titles do not imply newer revisions. No per-screen modification timestamps were returned.

| Screen | Screen ID | Screenshot dimensions | HTML | Screenshot |
|---|---|---|---|---|
| image.png | `15301958397873624784` | 1220×238 | Không có HTML | [PNG](15301958397873624784/screen.png) |
| W01 Sign in | `518f46085b834b8ca3012bebfce57f04` | 2560×2048 | [HTML](518f46085b834b8ca3012bebfce57f04/screen.html) | [PNG](518f46085b834b8ca3012bebfce57f04/screen.png) |
| W01 Sign in (#login) | `ca797a30d9a14f298953483347c4546f` | 2560×2048 | [HTML](ca797a30d9a14f298953483347c4546f/screen.html) | [JPG](ca797a30d9a14f298953483347c4546f/screen.jpg) |
| W02 Skills Store | `12113535d43548f8ac089a41897ef8b9` | 2560×2048 | [HTML](12113535d43548f8ac089a41897ef8b9/screen.html) | [JPG](12113535d43548f8ac089a41897ef8b9/screen.jpg) |
| W02 Skills Store (#store) | `e921a2c10a034eca80e4af20e760ad1f` | 2560×2048 | [HTML](e921a2c10a034eca80e4af20e760ad1f/screen.html) | [JPG](e921a2c10a034eca80e4af20e760ad1f/screen.jpg) |
| W03 Skill detail | `e2a4a81be76f47678beb4b0976b8bc55` | 2560×2048 | [HTML](e2a4a81be76f47678beb4b0976b8bc55/screen.html) | [PNG](e2a4a81be76f47678beb4b0976b8bc55/screen.png) |
| W03 Skill detail (#detail) | `7dcd2666d80d43dd8ae20556de775abc` | 2560×2048 | [HTML](7dcd2666d80d43dd8ae20556de775abc/screen.html) | [JPG](7dcd2666d80d43dd8ae20556de775abc/screen.jpg) |
| W04 Install skill | `8ba8f16c850b46d28e693af308e555ae` | 2560×2048 | [HTML](8ba8f16c850b46d28e693af308e555ae/screen.html) | [PNG](8ba8f16c850b46d28e693af308e555ae/screen.png) |
| W04 Install skill (#install) | `a3cca1f7d09646bca0f0a74a0fbbc1c2` | 2560×2048 | [HTML](a3cca1f7d09646bca0f0a74a0fbbc1c2/screen.html) | [JPG](a3cca1f7d09646bca0f0a74a0fbbc1c2/screen.jpg) |
| W05 AIPro MCP check | `95909941e1ca4fcda5e927598d08ed53` | 2560×2128 | [HTML](95909941e1ca4fcda5e927598d08ed53/screen.html) | [PNG](95909941e1ca4fcda5e927598d08ed53/screen.png) |
| W05 AIPro MCP check (#mcp) | `e297f1843ce04ab9a2189dd690f7f7ee` | 2560×2288 | [HTML](e297f1843ce04ab9a2189dd690f7f7ee/screen.html) | [JPG](e297f1843ce04ab9a2189dd690f7f7ee/screen.jpg) |
| W06 Installation result | `fa378c55ac4f40e9beba11b2b19bbd2b` | 2560×2048 | [HTML](fa378c55ac4f40e9beba11b2b19bbd2b/screen.html) | [PNG](fa378c55ac4f40e9beba11b2b19bbd2b/screen.png) |
| W06 Installation result (#result) | `727ccba2e7f14f37abce39cfcb41ad8d` | 2560×2048 | [HTML](727ccba2e7f14f37abce39cfcb41ad8d/screen.html) | [JPG](727ccba2e7f14f37abce39cfcb41ad8d/screen.jpg) |
| W07 Library | `6f5a4b08c21c4b4e98da2eb105bb29a3` | 2560×2048 | [HTML](6f5a4b08c21c4b4e98da2eb105bb29a3/screen.html) | [PNG](6f5a4b08c21c4b4e98da2eb105bb29a3/screen.png) |
| W07 Library (#library) | `c4c7355ac0aa416eb275e610be3e916a` | 2560×2048 | [HTML](c4c7355ac0aa416eb275e610be3e916a/screen.html) | [JPG](c4c7355ac0aa416eb275e610be3e916a/screen.jpg) |
| W08 Workspace and preset (#workspace) | `b6fe75bbad4240a69b78d5c5d5ec99af` | 2560×2048 | [HTML](b6fe75bbad4240a69b78d5c5d5ec99af/screen.html) | [PNG](b6fe75bbad4240a69b78d5c5d5ec99af/screen.png) |
| W08 Workspace and preset (#workspace) | `629741bbcca2475fabdd7b80e0f29c77` | 2560×2048 | [HTML](629741bbcca2475fabdd7b80e0f29c77/screen.html) | [JPG](629741bbcca2475fabdd7b80e0f29c77/screen.jpg) |
| W09 Update review | `634184e5dd9d48889bb8659b67932d56` | 2560×2048 | [HTML](634184e5dd9d48889bb8659b67932d56/screen.html) | [PNG](634184e5dd9d48889bb8659b67932d56/screen.png) |
| W09 Update review (#update) | `47e723baede14b0cb331426b055b6f8f` | 2560×2048 | [HTML](47e723baede14b0cb331426b055b6f8f/screen.html) | [JPG](47e723baede14b0cb331426b055b6f8f/screen.jpg) |
| W10 Offline Library | `98709e85496747fab8697a4f69e4c441` | 2880×2048 | [HTML](98709e85496747fab8697a4f69e4c441/screen.html) | [PNG](98709e85496747fab8697a4f69e4c441/screen.png) |
| W10 Offline Library (#offline) | `9c91f3d8eaae489e887790fa7bfd3113` | 2560×2048 | [HTML](9c91f3d8eaae489e887790fa7bfd3113/screen.html) | [JPG](9c91f3d8eaae489e887790fa7bfd3113/screen.jpg) |
| W11 Backup - local snapshots | `a88f85f148a248648319965997ff478e` | 2560×2048 | [HTML](a88f85f148a248648319965997ff478e/screen.html) | [PNG](a88f85f148a248648319965997ff478e/screen.png) |
| W11 Backup - local snapshots (#backup) | `9f7fa1d49a4a468bb88a18dc58def5f1` | 2560×2048 | [HTML](9f7fa1d49a4a468bb88a18dc58def5f1/screen.html) | [JPG](9f7fa1d49a4a468bb88a18dc58def5f1/screen.jpg) |
| W12 Settings | `130b37c6484947239f62b2802e1b7dea` | 2880×2118 | [HTML](130b37c6484947239f62b2802e1b7dea/screen.html) | [PNG](130b37c6484947239f62b2802e1b7dea/screen.png) |
| W12 Settings (#settings) | `662d6089aed24f248081b81a4a9fb2ed` | 2560×2048 | [HTML](662d6089aed24f248081b81a4a9fb2ed/screen.html) | [JPG](662d6089aed24f248081b81a4a9fb2ed/screen.jpg) |
| W13 Remove skill | `dce178ca8f44473b862e3d2da3d4c750` | 2560×2048 | [HTML](dce178ca8f44473b862e3d2da3d4c750/screen.html) | [PNG](dce178ca8f44473b862e3d2da3d4c750/screen.png) |
| W13 Remove skill (#remove) | `7f90de67b26746b3bb047c078b4e9fbe` | 2560×2048 | [HTML](7f90de67b26746b3bb047c078b4e9fbe/screen.html) | [JPG](7f90de67b26746b3bb047c078b4e9fbe/screen.jpg) |
| W14 Dashboard | `3192846cd64643279bae03f34f7dcddf` | 2560×2048 | [HTML](3192846cd64643279bae03f34f7dcddf/screen.html) | [PNG](3192846cd64643279bae03f34f7dcddf/screen.png) |
| W14 Dashboard (#dashboard) | `da58f733eb074f4aa714cb22457e12c8` | 2560×2048 | [HTML](da58f733eb074f4aa714cb22457e12c8/screen.html) | [JPG](da58f733eb074f4aa714cb22457e12c8/screen.jpg) |

