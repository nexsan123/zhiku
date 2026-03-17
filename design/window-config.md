# Window Configuration Specification

> 智库桌面窗口配置
> This file is the sole reference for @coder-be to update `tauri.conf.json` window settings.

---

## Window Properties

| Property | Value | Rationale |
|----------|-------|-----------|
| **Default width** | 1440px | Accommodates sidebar (240px) + map content comfortably, standard 16:10 base |
| **Default height** | 900px | 16:10-ish ratio, fits map projection well |
| **Minimum width** | 1024px | Sidebar collapsed (72px) + 952px content = minimum usable map |
| **Minimum height** | 640px | TitleBar (52px) + content (556px) + StatusBar (32px) = minimum |
| **Decorations** | `false` | Custom titlebar (see TitleBar in app-layout.md) |
| **Transparent** | `false` | Opaque window, glassmorphism is internal CSS only |
| **Resizable** | `true` | User can resize freely above minimums |
| **Fullscreenable** | `true` | Allow fullscreen via window control or F11 |
| **Center** | `true` | Window starts centered on primary monitor |
| **Title** | "智库" | Window title for OS taskbar/alt-tab |

## Tauri Config Mapping

```json
{
  "app": {
    "windows": [
      {
        "title": "智库",
        "width": 1440,
        "height": 900,
        "minWidth": 1024,
        "minHeight": 640,
        "decorations": false,
        "transparent": false,
        "resizable": true,
        "fullscreen": false,
        "center": true
      }
    ]
  }
}
```

## Custom TitleBar Implications

Since `decorations: false`, the app must implement:

1. **Window drag**: `-webkit-app-region: drag` on TitleBar component
2. **Window controls**: Minimize / Maximize / Close buttons calling Tauri window API
3. **Double-click titlebar**: Toggle maximize (standard Windows behavior)

Window control button styling is defined in `layouts/app-layout.md` TitleBar section.

## Notes

- No `alwaysOnTop` — 智库 is a normal window
- No custom window shadow — Windows 11 provides native shadow for non-decorated windows
- Position memory (save/restore last position) is a Phase 2+ enhancement
