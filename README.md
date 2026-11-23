# Norvane PiP

A cross-platform Tauri v2 desktop Picture-in-Picture wrapper for [Norvane Idle](https://ro-idle.pages.dev/).

## E2E Demo

See [docs/e2e-demo.mp4](docs/e2e-demo.mp4) for an end-to-end macOS recording demonstrating compact PiP mode, HUD toggle (compact & expanded), Always-on-Top pin toggle, and compact/expanded window switching.

## Features

- **Native Picture-in-Picture look & feel**: Frameless edge-to-edge window (`440×275`, min `280×180`) with native macOS rounded corners, drop shadow, and hidden traffic-light buttons (frameless + shadow on Windows/Linux). Opens anchored in the bottom-right corner of the active display's work area.
- **Hover-revealed glass PiP controls & drag bar**: Unhovered, only the game canvas is shown. Hovering reveals a translucent top bar with circular close/minimize buttons, a center drag handle (double-click to toggle compact/expanded size), and quick toggles for HUD, Always-on-Top, Workspaces, and Size.
- **Auto sticky to screen borders**: Dragging the PiP window in compact mode smoothly glides it to the nearest screen border (left, right, top, or bottom, 16px inset respecting menu bar / Dock / taskbar), preserving your position along that edge and magnetically locking into corners when within 48px of an adjacent edge. Toggleable via `View -> Sticky to Screen Borders`.
- **macOS workspace visibility**: Enabled by default via Tauri's `set_visible_on_all_workspaces(true)` plus macOS AppKit `NSWindowCollectionBehavior` (`CanJoinAllSpaces | FullScreenAuxiliary`, clearing mutually exclusive `MoveToActiveSpace` / `FullScreenPrimary` flags).
- **Persistent preferences**: Remembers all toggles (`Hide Game HUD`, `Always on Top`, `Visible on All Workspaces`, `Sticky to Screen Borders`, `Compact PiP / Expanded`), custom window sizes for both modes, and last window position across restarts (stored atomically in the OS app config directory as `preferences.json`, with multi-monitor bounds validation).

## Shortcuts & Menu Controls

| Action | Shortcut |
| --- | --- |
| Hide / Show Game HUD | `Cmd/Ctrl + Shift + H` |
| Always on Top | `Cmd/Ctrl + Shift + T` |
| Visible on All Workspaces | `Cmd/Ctrl + Shift + W` |
| Sticky to Screen Borders | `Cmd/Ctrl + Shift + S` |
| Compact PiP / Expanded Size | `Cmd/Ctrl + Shift + P` |
| Reload Game | `Cmd/Ctrl + R` |
| Keyboard Shortcuts Panel | `Cmd/Ctrl + ?` (`Cmd/Ctrl + Shift + /`) |

## Development

Prerequisites: Rust (`>= 1.90`), Node.js, and platform WebView prerequisites for Tauri v2.

```bash
npm install
npm run dev
```

Build a distributable bundle:

```bash
npm run build
```

Run checks and unit tests:

```bash
npm run check
npm test
```

## Validation Status

- **Verified on macOS**: `node --check src-tauri/src/pip_overlay.js`, `cargo clippy --all-targets -- -D warnings`, `cargo test` (8 unit tests covering state transitions, corner snapping, URL/sync serialization, and macOS `NSWindowCollectionBehavior` flags), `cargo build`, and native binary startup smoke check.
- **Still to verify interactively**: visual HUD hide/show behavior across all in-game states (including boss fights and modals), macOS full-screen Space overlay behavior across third-party full-screen apps, and Windows/Linux packaging and workspace behavior.
