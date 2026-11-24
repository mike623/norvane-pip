(function () {
  if (window.self !== window.top) return;
  if (window.__norvanePipInitialized) return;
  window.__norvanePipInitialized = true;

  const STORAGE_HUD_KEY = "norvane-pip-hud-hidden";
  const STYLE_ID = "norvane-pip-style";
  const CHROME_ID = "norvane-pip-chrome";
  const TOAST_ID = "norvane-pip-toast";
  const SHORTCUTS_ID = "norvane-pip-shortcuts";

  const state = {
    hudHidden: true,
    alwaysOnTop: true,
    visibleOnAllWorkspaces: true,
    compactMode: true,
    snapToCorners: true,
    platform: "unknown",
  };

  let toastTimer = null;

  const ICONS = {
    close:
      '<svg viewBox="0 0 12 12" width="10" height="10" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M2.5 2.5l7 7M9.5 2.5l-7 7"/></svg>',
    minimize:
      '<svg viewBox="0 0 12 12" width="10" height="10" fill="none" stroke="currentColor" stroke-width="1.8" stroke-linecap="round"><path d="M2.5 6h7"/></svg>',
    eyeOff:
      '<svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M2.5 2.5l11 11"/><path d="M6.4 6.5a2 2 0 0 0 2.7 2.7"/><path d="M9.9 4.2A7.2 7.2 0 0 1 14.5 8c-.8 1.5-2.1 2.7-3.7 3.3M6.1 11.8A7.2 7.2 0 0 1 1.5 8c.7-1.3 1.7-2.4 3-3.1"/></svg>',
    eyeOn:
      '<svg viewBox="0 0 16 16" width="13" height="13" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M1.5 8s2.5-4.2 6.5-4.2S14.5 8 14.5 8s-2.5 4.2-6.5 4.2S1.5 8 1.5 8z"/><circle cx="8" cy="8" r="2"/></svg>',
    pinOn:
      '<svg viewBox="0 0 16 16" width="12" height="12" fill="currentColor" stroke="currentColor" stroke-width="1.3" stroke-linecap="round" stroke-linejoin="round"><path d="M9.8 2.6l3.6 3.6-1.2 1.2-1-.3-2.6 2.6.5 1.8-1 1-2.4-2.4-1.2 1.2-1.9 1.9-.6-.6 1.9-1.9 1.2-1.2-2.4-2.4 1-1 1.8.5 2.6-2.6-.3-1z"/></svg>',
    pinOff:
      '<svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><path d="M2.5 2.5l11 11"/><path d="M9.8 2.6l3.6 3.6-1.2 1.2-1-.3-1.4 1.4"/><path d="M6.4 6.4l-1.9-.5-1 1 2.4 2.4-3.1 3.1.6.6 3.1-3.1 2.4 2.4 1-1-.5-1.9"/></svg>',
    spaces:
      '<svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="3" width="5" height="4.5" rx="1"/><rect x="9" y="3" width="5" height="4.5" rx="1"/><rect x="2" y="9" width="5" height="4.5" rx="1"/><rect x="9" y="9" width="5" height="4.5" rx="1"/></svg>',
    expand:
      '<svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><path d="M9.5 2.5h4v4M13.5 2.5L9 7M6.5 13.5h-4v-4M2.5 13.5L7 9"/></svg>',
    compact:
      '<svg viewBox="0 0 16 16" width="12" height="12" fill="none" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" stroke-linejoin="round"><rect x="2" y="2.5" width="12" height="11" rx="2"/><rect x="7.5" y="7" width="4.5" height="4.5" rx="1" fill="currentColor" stroke="none"/></svg>',
  };

  const CSS = `
    :root {
      --np-bar-h: 34px;
    }

    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) {
      --np-bar-h: 0px;
    }

    /* When game HUD is visible (or on start/town screens), reserve a slim 34px top bar
       so the PiP drag handle and controls never overlap Norvane Idle's header buttons. */
    #root {
      padding-top: calc(12px + var(--np-bar-h)) !important;
    }

    .app-shell.caca-pc .layout-flutuante {
      --topo-do-layout: calc(56px + var(--np-bar-h)) !important;
    }

    .app-shell.caca-pc .app-actions {
      top: calc(52px + var(--np-bar-h)) !important;
      max-height: calc(100vh - 62px - var(--np-bar-h)) !important;
    }

    .app-shell.caca-cheia {
      --topo-da-caca: calc(50px + var(--np-bar-h) + env(safe-area-inset-top, 0px)) !important;
    }

    .app-shell.caca-cheia > .app-header {
      top: var(--np-bar-h) !important;
    }

    .modal-backdrop {
      top: var(--np-bar-h) !important;
    }

    /* Hide Norvane Idle HUD overlays whenever an active hunting map is mounted —
       both compact mode (.sala-de-caca .campo) and expanded desktop mode (.layout-flutuante-mapa .campo) —
       so start screens, character creation, town panels, and modals remain usable. */
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) #root {
      padding: 0 !important;
    }

    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .app-header,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .app-actions,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .abas-do-celular,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .canto-da-caca,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .missao-na-caca,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .guia-recolhido,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .guia-balao,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .guia-foco,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .guia-veu,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .guia-veu-parte,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .game-layout > .character-panel,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .game-layout > .coluna-direita,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .game-layout > .left-column,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .sala-de-caca .zone-label,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .sala-de-caca .zone-title,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .sala-de-caca .room-actions,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .janela-flutuante,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .doca-do-layout,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .tira-de-janelas,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .campo-buffs,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .campo-legenda,
    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .barra-do-chefe {
      display: none !important;
    }

    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .layout-flutuante {
      --topo-do-layout: 0px !important;
      --coluna-da-direita: 0px !important;
      top: 0 !important;
      left: 0 !important;
      right: 0 !important;
      bottom: 0 !important;
    }

    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .game-layout > .hunt-column {
      display: flex !important;
      position: fixed !important;
      inset: 0 !important;
      z-index: 1 !important;
      width: 100vw !important;
      height: 100vh !important;
      max-height: none !important;
      margin: 0 !important;
      padding: 0 !important;
      overflow: hidden !important;
    }

    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .sala-de-caca {
      position: relative !important;
      flex: 1 1 auto !important;
      width: 100% !important;
      height: 100% !important;
      margin: 0 !important;
      padding: 0 !important;
      border: 0 !important;
      border-radius: 0 !important;
      box-shadow: none !important;
      background: #0d0b08 !important;
    }

    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) .sala-de-caca > .campo {
      position: absolute !important;
      inset: 0 !important;
      width: 100% !important;
      height: 100% !important;
      padding: 0 !important;
      border: 0 !important;
      border-radius: 0 !important;
    }

    /* Top PiP bar mounted outside React's #root.
       Docked as a slim 34px bar when HUD is visible; hover-only floating overlay when HUD is hidden. */
    #${CHROME_ID} {
      position: fixed;
      top: 0;
      left: 0;
      right: 0;
      height: 34px;
      z-index: 2147483647;
      display: flex;
      align-items: center;
      justify-content: space-between;
      padding: 0 8px;
      background: rgba(20, 18, 15, 0.94);
      border-bottom: 0.5px solid rgba(255, 255, 255, 0.12);
      backdrop-filter: blur(14px) saturate(1.4);
      -webkit-backdrop-filter: blur(14px) saturate(1.4);
      opacity: 1;
      transition: opacity 180ms cubic-bezier(0.22, 1, 0.36, 1), background 180ms ease;
      pointer-events: auto;
      font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", Roboto, sans-serif;
      user-select: none;
      -webkit-user-select: none;
    }

    html.norvane-pip-hide-hud:has(:is(.sala-de-caca, .layout-flutuante-mapa) .campo) #${CHROME_ID} {
      height: 44px;
      align-items: flex-start;
      padding: 8px 10px 0;
      background: linear-gradient(
        180deg,
        rgba(0, 0, 0, 0.56) 0%,
        rgba(0, 0, 0, 0.22) 60%,
        rgba(0, 0, 0, 0) 100%
      );
      border-bottom: 0;
      backdrop-filter: none;
      -webkit-backdrop-filter: none;
      opacity: 0;
      pointer-events: none;
    }

    /* Subtle hairline inner frame like native macOS PiP */
    html::after {
      content: "";
      position: fixed;
      inset: 0;
      pointer-events: none;
      z-index: 2147483646;
      box-shadow: inset 0 0 0 0.5px rgba(255, 255, 255, 0.16);
    }

    html.norvane-pip-hover #${CHROME_ID},
    #${CHROME_ID}:hover,
    #${CHROME_ID}:focus-within {
      opacity: 1 !important;
    }

    #${CHROME_ID} .np-group {
      display: flex;
      align-items: center;
      gap: 6px;
      pointer-events: auto;
    }

    #${CHROME_ID} .np-drag {
      flex: 1 1 auto;
      height: 28px;
      margin: 0 8px;
      display: flex;
      align-items: center;
      justify-content: center;
      cursor: grab;
      pointer-events: auto;
    }

    #${CHROME_ID} .np-drag:active {
      cursor: grabbing;
    }

    #${CHROME_ID} .np-drag-pill {
      width: 36px;
      height: 4px;
      border-radius: 999px;
      background: rgba(255, 255, 255, 0.34);
      transition: background 140ms ease, width 140ms ease;
    }

    #${CHROME_ID} .np-drag:hover .np-drag-pill {
      width: 44px;
      background: rgba(255, 255, 255, 0.62);
    }

    #${CHROME_ID} button {
      position: relative;
      appearance: none;
      border: 0.5px solid rgba(255, 255, 255, 0.16);
      background: rgba(28, 28, 30, 0.68);
      color: rgba(255, 255, 255, 0.9);
      width: 26px;
      height: 26px;
      padding: 0;
      border-radius: 999px;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      backdrop-filter: blur(14px) saturate(1.4);
      -webkit-backdrop-filter: blur(14px) saturate(1.4);
      box-shadow: 0 2px 8px rgba(0, 0, 0, 0.36);
      transition: background 120ms ease, color 120ms ease, transform 100ms ease, border-color 120ms ease;
    }

    #${CHROME_ID} .np-drag {
      position: relative;
    }

    #${CHROME_ID} [data-tooltip]::after {
      content: attr(data-tooltip);
      position: absolute;
      top: calc(100% + 7px);
      z-index: 2147483647;
      padding: 4px 8px;
      border-radius: 6px;
      background: rgba(20, 20, 22, 0.94);
      border: 0.5px solid rgba(255, 255, 255, 0.18);
      color: rgba(255, 255, 255, 0.94);
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.48);
      font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", Roboto, sans-serif;
      font-size: 11px;
      font-weight: 500;
      line-height: 1.25;
      white-space: nowrap;
      pointer-events: none;
      opacity: 0;
      transform: translateY(-2px);
      transition: opacity 140ms ease 140ms, transform 140ms ease 140ms;
    }

    #${CHROME_ID} .np-group-left [data-tooltip]::after {
      left: 0;
    }

    #${CHROME_ID} .np-group-right [data-tooltip]::after {
      right: 0;
    }

    #${CHROME_ID} .np-drag[data-tooltip]::after {
      left: 50%;
      transform: translateX(-50%) translateY(-2px);
    }

    #${CHROME_ID} [data-tooltip]:hover::after,
    #${CHROME_ID} [data-tooltip]:focus-visible::after {
      opacity: 1;
      transform: translateY(0);
    }

    #${CHROME_ID} .np-drag[data-tooltip]:hover::after {
      opacity: 1;
      transform: translateX(-50%) translateY(0);
    }

    #${CHROME_ID} button:hover {
      background: rgba(54, 54, 58, 0.88);
      border-color: rgba(255, 255, 255, 0.32);
      color: #ffffff;
    }

    #${CHROME_ID} button:active {
      transform: scale(0.93);
    }

    #${CHROME_ID} button[data-action="close"]:hover {
      background: rgba(255, 69, 58, 0.88);
      border-color: rgba(255, 120, 112, 0.65);
    }

    #${CHROME_ID} button[aria-pressed="true"] {
      background: rgba(240, 208, 120, 0.28);
      border-color: rgba(240, 208, 120, 0.65);
      color: #f6df96;
    }

    #${CHROME_ID} button.hidden {
      display: none;
    }

    #${TOAST_ID} {
      position: fixed;
      left: 50%;
      bottom: 12px;
      transform: translateX(-50%) translateY(6px);
      z-index: 2147483647;
      padding: 5px 12px;
      border-radius: 999px;
      background: rgba(28, 28, 30, 0.84);
      border: 0.5px solid rgba(255, 255, 255, 0.18);
      color: rgba(255, 255, 255, 0.94);
      backdrop-filter: blur(14px) saturate(1.4);
      -webkit-backdrop-filter: blur(14px) saturate(1.4);
      box-shadow: 0 4px 14px rgba(0, 0, 0, 0.42);
      font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", Roboto, sans-serif;
      font-size: 11px;
      font-weight: 500;
      letter-spacing: 0.01em;
      pointer-events: none;
      opacity: 0;
      transition: opacity 160ms ease, transform 160ms ease;
      white-space: nowrap;
    }

    #${TOAST_ID}.show {
      opacity: 1;
      transform: translateX(-50%) translateY(0);
    }

    #${SHORTCUTS_ID} {
      position: fixed;
      inset: 0;
      z-index: 2147483647;
      display: none;
      align-items: center;
      justify-content: center;
      padding: 12px;
      background: rgba(0, 0, 0, 0.45);
      backdrop-filter: blur(4px);
      -webkit-backdrop-filter: blur(4px);
      font-family: -apple-system, BlinkMacSystemFont, "SF Pro Text", "Segoe UI", Roboto, sans-serif;
    }

    #${SHORTCUTS_ID}.open {
      display: flex;
    }

    #${SHORTCUTS_ID} .np-sc-card {
      width: min(320px, 100%);
      max-height: 100%;
      overflow-y: auto;
      padding: 12px 14px;
      border-radius: 12px;
      background: rgba(24, 24, 27, 0.92);
      border: 0.5px solid rgba(255, 255, 255, 0.18);
      box-shadow: 0 14px 36px rgba(0, 0, 0, 0.6);
      backdrop-filter: blur(18px) saturate(1.5);
      -webkit-backdrop-filter: blur(18px) saturate(1.5);
      color: rgba(255, 255, 255, 0.92);
    }

    #${SHORTCUTS_ID} .np-sc-head {
      display: flex;
      align-items: center;
      justify-content: space-between;
      margin-bottom: 8px;
      padding-bottom: 6px;
      border-bottom: 0.5px solid rgba(255, 255, 255, 0.12);
    }

    #${SHORTCUTS_ID} .np-sc-title {
      font-size: 12px;
      font-weight: 600;
      letter-spacing: 0.02em;
      color: #f6df96;
    }

    #${SHORTCUTS_ID} .np-sc-close {
      appearance: none;
      border: 0.5px solid rgba(255, 255, 255, 0.14);
      background: rgba(255, 255, 255, 0.08);
      color: rgba(255, 255, 255, 0.85);
      width: 20px;
      height: 20px;
      border-radius: 999px;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      justify-content: center;
      padding: 0;
    }

    #${SHORTCUTS_ID} .np-sc-close:hover {
      background: rgba(255, 255, 255, 0.18);
      color: #ffffff;
    }

    #${SHORTCUTS_ID} .np-sc-list {
      display: flex;
      flex-direction: column;
      gap: 5px;
    }

    #${SHORTCUTS_ID} .np-sc-row {
      display: flex;
      align-items: center;
      justify-content: space-between;
      gap: 10px;
      font-size: 11px;
      line-height: 1.3;
    }

    #${SHORTCUTS_ID} .np-sc-label {
      color: rgba(255, 255, 255, 0.82);
    }

    #${SHORTCUTS_ID} kbd {
      font-family: ui-monospace, SFMono-Regular, Menlo, monospace;
      font-size: 10px;
      font-weight: 600;
      padding: 2px 6px;
      border-radius: 5px;
      background: rgba(255, 255, 255, 0.1);
      border: 0.5px solid rgba(255, 255, 255, 0.18);
      color: rgba(255, 255, 255, 0.94);
      white-space: nowrap;
    }
  `;

  function invokeTauri(cmd, args) {
    const tauri = window.__TAURI__;
    if (tauri && tauri.core && typeof tauri.core.invoke === "function") {
      return tauri.core.invoke(cmd, args).catch(function (err) {
        console.warn("[Norvane PiP] invoke failed:", cmd, err);
        return null;
      });
    }
    return Promise.resolve(null);
  }

  function triggerResize() {
    window.requestAnimationFrame(function () {
      window.dispatchEvent(new Event("resize"));
      window.setTimeout(function () {
        window.dispatchEvent(new Event("resize"));
      }, 60);
    });
  }

  function showToast(message) {
    const toast = document.getElementById(TOAST_ID);
    if (!toast) return;
    toast.textContent = message;
    toast.classList.add("show");
    if (toastTimer) window.clearTimeout(toastTimer);
    toastTimer = window.setTimeout(function () {
      toast.classList.remove("show");
    }, 1250);
  }

  function readStoredHudHidden() {
    try {
      const val = window.localStorage.getItem(STORAGE_HUD_KEY);
      if (val === "1") return true;
      if (val === "0") return false;
      return null;
    } catch (_) {
      return null;
    }
  }

  function writeStoredHudHidden(hidden) {
    try {
      window.localStorage.setItem(STORAGE_HUD_KEY, hidden ? "1" : "0");
    } catch (_) {
      /* ignore storage errors */
    }
  }

  function shortcutHint(key) {
    return state.platform === "macos" ? "⌘⇧" + key : "Ctrl+Shift+" + key;
  }

  function renderShortcutsList() {
    const overlay = document.getElementById(SHORTCUTS_ID);
    if (!overlay) return;
    const list = overlay.querySelector(".np-sc-list");
    if (!list) return;

    const isMac = state.platform === "macos";
    const rows = [
      ["Hide / Show Game HUD", shortcutHint("H")],
      ["Always on Top", shortcutHint("T")],
      ["Visible on All Spaces", shortcutHint("W")],
      ["Sticky to Screen Borders", shortcutHint("S")],
      ["Compact PiP / Expand", shortcutHint("P")],
      ["Reload Game", isMac ? "⌘R" : "Ctrl+R"],
      ["Minimize / Close", isMac ? "⌘M / ⌘W" : "Ctrl+M / Ctrl+W"],
      ["Shortcuts Panel", isMac ? "⌘?" : "Ctrl+?"],
    ];

    list.innerHTML = rows
      .map(function (row) {
        return (
          '<div class="np-sc-row"><span class="np-sc-label">' +
          row[0] +
          "</span><kbd>" +
          row[1] +
          "</kbd></div>"
        );
      })
      .join("");
  }

  function toggleShortcutsPanel(forceOpen) {
    mountControls();
    const overlay = document.getElementById(SHORTCUTS_ID);
    if (!overlay) return;
    renderShortcutsList();
    const nextOpen =
      typeof forceOpen === "boolean" ? forceOpen : !overlay.classList.contains("open");
    overlay.classList.toggle("open", nextOpen);
  }

  function renderChromeState() {
    const chrome = document.getElementById(CHROME_ID);
    if (!chrome) return;

    const closeBtn = chrome.querySelector('[data-action="close"]');
    const minBtn = chrome.querySelector('[data-action="minimize"]');
    const hudBtn = chrome.querySelector('[data-action="hud"]');
    const pinBtn = chrome.querySelector('[data-action="pin"]');
    const spacesBtn = chrome.querySelector('[data-action="spaces"]');
    const sizeBtn = chrome.querySelector('[data-action="size"]');

    if (closeBtn) {
      const k = state.platform === "macos" ? "⌘W" : "Ctrl+W";
      closeBtn.dataset.tooltip = "Close window (" + k + ")";
    }
    if (minBtn) {
      const k = state.platform === "macos" ? "⌘M" : "Ctrl+M";
      minBtn.dataset.tooltip = "Minimize window (" + k + ")";
    }
    if (hudBtn) {
      hudBtn.setAttribute("aria-pressed", String(state.hudHidden));
      hudBtn.innerHTML = state.hudHidden ? ICONS.eyeOff : ICONS.eyeOn;
      hudBtn.dataset.tooltip = state.hudHidden
        ? "Show game HUD (" + shortcutHint("H") + ")"
        : "Hide game HUD (" + shortcutHint("H") + ")";
    }
    if (pinBtn) {
      pinBtn.setAttribute("aria-pressed", String(state.alwaysOnTop));
      pinBtn.innerHTML = state.alwaysOnTop ? ICONS.pinOn : ICONS.pinOff;
      pinBtn.dataset.tooltip = state.alwaysOnTop
        ? "Pinned on top — click to unpin (" + shortcutHint("T") + ")"
        : "Pin always on top (" + shortcutHint("T") + ")";
    }
    if (spacesBtn) {
      const showSpaces = state.platform === "macos" || state.platform === "linux";
      spacesBtn.classList.toggle("hidden", !showSpaces);
      spacesBtn.setAttribute("aria-pressed", String(state.visibleOnAllWorkspaces));
      spacesBtn.dataset.tooltip = state.visibleOnAllWorkspaces
        ? "Visible on all Spaces: On (" + shortcutHint("W") + ")"
        : "Visible on all Spaces: Off (" + shortcutHint("W") + ")";
    }
    if (sizeBtn) {
      sizeBtn.setAttribute("aria-pressed", String(!state.compactMode));
      sizeBtn.innerHTML = state.compactMode ? ICONS.expand : ICONS.compact;
      sizeBtn.dataset.tooltip = state.compactMode
        ? "Expand to full window (" + shortcutHint("P") + ")"
        : "Return to compact PiP (" + shortcutHint("P") + ")";
    }
  }

  function applyHudClass(hidden, announce) {
    state.hudHidden = Boolean(hidden);
    document.documentElement.classList.toggle("norvane-pip-hide-hud", state.hudHidden);
    writeStoredHudHidden(state.hudHidden);
    renderChromeState();
    triggerResize();
    if (announce) {
      showToast(state.hudHidden ? "HUD Hidden" : "HUD Visible");
    }
  }

  function applyRemoteState(next, announceMessage) {
    if (next && typeof next === "object") {
      if (typeof next.always_on_top === "boolean") {
        state.alwaysOnTop = next.always_on_top;
      }
      if (typeof next.visible_on_all_workspaces === "boolean") {
        state.visibleOnAllWorkspaces = next.visible_on_all_workspaces;
      }
      if (typeof next.compact_mode === "boolean") {
        state.compactMode = next.compact_mode;
      }
      if (typeof next.snap_to_corners === "boolean") {
        state.snapToCorners = next.snap_to_corners;
      }
      if (typeof next.platform === "string") {
        state.platform = next.platform;
      }
      if (typeof next.hud_hidden === "boolean") {
        applyHudClass(next.hud_hidden, false);
      } else {
        renderChromeState();
      }
    }
    if (announceMessage) {
      showToast(announceMessage);
    }
  }

  function toggleHudFromWebview() {
    const nextHidden = !state.hudHidden;
    applyHudClass(nextHidden, true);
    invokeTauri("set_hud_hidden", { hidden: nextHidden }).then(function (res) {
      if (res) applyRemoteState(res, null);
    });
  }

  function makeButton(cfg) {
    const btn = document.createElement("button");
    btn.type = "button";
    btn.dataset.action = cfg.action;
    btn.dataset.tooltip = cfg.title;
    btn.innerHTML = cfg.icon;
    btn.setAttribute("aria-label", cfg.ariaLabel || cfg.title);
    btn.setAttribute("aria-pressed", "false");
    btn.addEventListener("mousedown", function (ev) {
      ev.stopPropagation();
    });
    btn.addEventListener("click", function (ev) {
      ev.stopPropagation();
      cfg.onClick();
    });
    return btn;
  }

  function mountControls() {
    if (!document.documentElement) return;

    if (!document.getElementById(STYLE_ID)) {
      const style = document.createElement("style");
      style.id = STYLE_ID;
      style.textContent = CSS;
      (document.head || document.documentElement).appendChild(style);
    }

    if (!document.getElementById(CHROME_ID)) {
      const chrome = document.createElement("div");
      chrome.id = CHROME_ID;
      chrome.setAttribute("role", "toolbar");
      chrome.setAttribute("aria-label", "Norvane PiP Controls");

      const leftGroup = document.createElement("div");
      leftGroup.className = "np-group np-group-left";
      leftGroup.appendChild(
        makeButton({
          action: "close",
          icon: ICONS.close,
          title: "Close window",
          ariaLabel: "Close Window",
          onClick: function () {
            invokeTauri("close_window");
          },
        })
      );
      leftGroup.appendChild(
        makeButton({
          action: "minimize",
          icon: ICONS.minimize,
          title: "Minimize window",
          ariaLabel: "Minimize Window",
          onClick: function () {
            invokeTauri("minimize_window");
          },
        })
      );

      const dragZone = document.createElement("div");
      dragZone.className = "np-drag";
      dragZone.dataset.tooltip = "Drag to move · Double-click to resize";
      const dragPill = document.createElement("div");
      dragPill.className = "np-drag-pill";
      dragZone.appendChild(dragPill);
      dragZone.addEventListener("mousedown", function (ev) {
        if (ev.button !== 0 || ev.detail >= 2) return;
        ev.preventDefault();
        invokeTauri("start_pip_drag");
      });
      dragZone.addEventListener("dblclick", function (ev) {
        ev.preventDefault();
        invokeTauri("toggle_compact_size").then(function (res) {
          if (res) {
            applyRemoteState(
              res,
              res.compact_mode ? "Compact PiP Size" : "Expanded Size"
            );
          }
        });
      });

      const rightGroup = document.createElement("div");
      rightGroup.className = "np-group np-group-right";
      rightGroup.appendChild(
        makeButton({
          action: "hud",
          icon: ICONS.eyeOff,
          title: "Toggle Game HUD (Cmd/Ctrl+Shift+H)",
          ariaLabel: "Toggle Game HUD",
          onClick: toggleHudFromWebview,
        })
      );
      rightGroup.appendChild(
        makeButton({
          action: "pin",
          icon: ICONS.pinOn,
          title: "Unpin from Top (Cmd/Ctrl+Shift+T)",
          ariaLabel: "Toggle Always on Top",
          onClick: function () {
            invokeTauri("toggle_always_on_top").then(function (res) {
              if (res) {
                applyRemoteState(
                  res,
                  res.always_on_top ? "Always on Top: On" : "Always on Top: Off"
                );
              }
            });
          },
        })
      );
      rightGroup.appendChild(
        makeButton({
          action: "spaces",
          icon: ICONS.spaces,
          title: "Visible on All Workspaces (Cmd/Ctrl+Shift+W)",
          ariaLabel: "Visible on All Workspaces",
          onClick: function () {
            invokeTauri("toggle_all_workspaces").then(function (res) {
              if (res) {
                applyRemoteState(
                  res,
                  res.visible_on_all_workspaces
                    ? "All Workspaces: On"
                    : "All Workspaces: Off"
                );
              }
            });
          },
        })
      );
      rightGroup.appendChild(
        makeButton({
          action: "size",
          icon: ICONS.expand,
          title: "Toggle Compact PiP / Expanded Size (Cmd/Ctrl+Shift+P)",
          ariaLabel: "Toggle Compact PiP Size",
          onClick: function () {
            invokeTauri("toggle_compact_size").then(function (res) {
              if (res) {
                applyRemoteState(
                  res,
                  res.compact_mode ? "Compact PiP Size" : "Expanded Size"
                );
              }
            });
          },
        })
      );

      chrome.appendChild(leftGroup);
      chrome.appendChild(dragZone);
      chrome.appendChild(rightGroup);
      document.documentElement.appendChild(chrome);
    }

    if (!document.getElementById(TOAST_ID)) {
      const toast = document.createElement("div");
      toast.id = TOAST_ID;
      toast.setAttribute("aria-live", "polite");
      document.documentElement.appendChild(toast);
    }

    if (!document.getElementById(SHORTCUTS_ID)) {
      const overlay = document.createElement("div");
      overlay.id = SHORTCUTS_ID;
      overlay.setAttribute("role", "dialog");
      overlay.setAttribute("aria-modal", "true");
      overlay.setAttribute("aria-label", "Norvane PiP Keyboard Shortcuts");
      overlay.innerHTML =
        '<div class="np-sc-card">' +
        '<div class="np-sc-head">' +
        '<span class="np-sc-title">Norvane PiP Shortcuts</span>' +
        '<button type="button" class="np-sc-close" aria-label="Close shortcuts">' +
        ICONS.close +
        "</button>" +
        "</div>" +
        '<div class="np-sc-list"></div>' +
        "</div>";

      overlay.addEventListener("click", function (ev) {
        if (ev.target === overlay) toggleShortcutsPanel(false);
      });
      const closeBtn = overlay.querySelector(".np-sc-close");
      if (closeBtn) {
        closeBtn.addEventListener("click", function () {
          toggleShortcutsPanel(false);
        });
      }
      document.documentElement.appendChild(overlay);
      renderShortcutsList();
    }

    renderChromeState();
  }

  // Reveal the native PiP overlay controls whenever the pointer hovers the window.
  let hoverLeaveTimer = null;
  function setHover(active) {
    if (hoverLeaveTimer) {
      window.clearTimeout(hoverLeaveTimer);
      hoverLeaveTimer = null;
    }
    if (active) {
      document.documentElement.classList.add("norvane-pip-hover");
    } else {
      hoverLeaveTimer = window.setTimeout(function () {
        document.documentElement.classList.remove("norvane-pip-hover");
      }, 120);
    }
  }

  document.addEventListener("pointerenter", function () {
    setHover(true);
  });
  document.addEventListener("pointermove", function () {
    if (!document.documentElement.classList.contains("norvane-pip-hover")) {
      setHover(true);
    }
  });
  document.addEventListener("pointerleave", function () {
    setHover(false);
  });
  window.addEventListener("blur", function () {
    setHover(false);
  });

  window.addEventListener(
    "keydown",
    function (ev) {
      if (ev.key === "Escape") {
        const overlay = document.getElementById(SHORTCUTS_ID);
        if (overlay && overlay.classList.contains("open")) {
          ev.preventDefault();
          toggleShortcutsPanel(false);
        }
      }
    },
    true
  );

  // Public bridge called from Rust menu items via `window.eval(...)`.
  window.__norvanePip = {
    setHudHidden: function (hidden, announce) {
      mountControls();
      applyHudClass(Boolean(hidden), announce !== false);
    },
    toggleHud: function () {
      mountControls();
      toggleHudFromWebview();
    },
    toggleShortcuts: function () {
      toggleShortcutsPanel();
    },
    syncState: function (next, announceMessage) {
      mountControls();
      applyRemoteState(next, announceMessage || null);
      renderShortcutsList();
    },
  };

  function boot() {
    mountControls();
    invokeTauri("get_pip_state").then(function (remote) {
      if (remote) {
        applyRemoteState(remote, null);
        return;
      }
      const storedHidden = readStoredHudHidden();
      if (storedHidden !== null) {
        applyHudClass(storedHidden, false);
      }
    });
  }

  if (document.readyState === "loading") {
    document.addEventListener("DOMContentLoaded", boot, { once: true });
  } else {
    boot();
  }
})();
