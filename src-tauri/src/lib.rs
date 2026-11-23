#[cfg(target_os = "macos")]
mod macos;

use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc, Mutex,
    },
};
use tauri::{
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu},
    AppHandle, LogicalSize, Manager, PhysicalPosition, Position, Size, State, WebviewUrl,
    WebviewWindow, WebviewWindowBuilder, WindowEvent,
};

pub const GAME_URL: &str = "https://ro-idle.pages.dev/";
pub const MAIN_WINDOW_LABEL: &str = "main";
pub const DEFAULT_COMPACT_SIZE: (f64, f64) = (440.0, 275.0);
pub const DEFAULT_EXPANDED_SIZE: (f64, f64) = (1100.0, 720.0);
pub const MIN_WINDOW_SIZE: (f64, f64) = (280.0, 180.0);
pub const BORDER_MARGIN_LOGICAL: f64 = 16.0;
pub const CORNER_MAGNET_LOGICAL: f64 = 48.0;
const PREFS_FILE_NAME: &str = "preferences.json";

const PIP_OVERLAY_SCRIPT: &str = include_str!("pip_overlay.js");

const MENU_TOGGLE_HUD: &str = "view.toggle_hud";
const MENU_ALWAYS_ON_TOP: &str = "view.always_on_top";
const MENU_ALL_WORKSPACES: &str = "view.all_workspaces";
const MENU_STICKY_BORDERS: &str = "view.sticky_borders";
const MENU_COMPACT_SIZE: &str = "view.compact_size";
const MENU_RESET_COMPACT_SIZE: &str = "view.reset_compact_size";
const MENU_RELOAD_GAME: &str = "view.reload_game";
const MENU_SHORTCUTS: &str = "view.shortcuts";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenCorner {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkAreaRect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

/// Computes the target top-left pixel position for a window anchored to a screen corner
/// within `work_area` (which excludes OS menu bar / Dock / taskbar), inset by `margin`.
pub fn corner_position(
    work_area: WorkAreaRect,
    win_size: (i32, i32),
    margin: i32,
    corner: ScreenCorner,
) -> (i32, i32) {
    let left_x = work_area.x + margin;
    let right_x = (work_area.x + work_area.width - win_size.0 - margin).max(left_x);
    let top_y = work_area.y + margin;
    let bottom_y = (work_area.y + work_area.height - win_size.1 - margin).max(top_y);

    match corner {
        ScreenCorner::TopLeft => (left_x, top_y),
        ScreenCorner::TopRight => (right_x, top_y),
        ScreenCorner::BottomLeft => (left_x, bottom_y),
        ScreenCorner::BottomRight => (right_x, bottom_y),
    }
}

/// Sticks the PiP window to the nearest screen border (left, right, top, or bottom)
/// within `work_area` inset by `margin`, while preserving its position along that edge
/// and magnetically locking into a corner when within `corner_magnet` of an adjacent edge.
pub fn sticky_border_position(
    work_area: WorkAreaRect,
    win_pos: (i32, i32),
    win_size: (i32, i32),
    margin: i32,
    corner_magnet: i32,
) -> (i32, i32) {
    let min_x = work_area.x + margin;
    let max_x = (work_area.x + work_area.width - win_size.0 - margin).max(min_x);
    let min_y = work_area.y + margin;
    let max_y = (work_area.y + work_area.height - win_size.1 - margin).max(min_y);

    let clamped_x = win_pos.0.clamp(min_x, max_x);
    let clamped_y = win_pos.1.clamp(min_y, max_y);

    let dist_left = clamped_x - min_x;
    let dist_right = max_x - clamped_x;
    let dist_top = clamped_y - min_y;
    let dist_bottom = max_y - clamped_y;

    let (min_h, snap_x) = if dist_left <= dist_right {
        (dist_left, min_x)
    } else {
        (dist_right, max_x)
    };
    let (min_v, snap_y) = if dist_top <= dist_bottom {
        (dist_top, min_y)
    } else {
        (dist_bottom, max_y)
    };

    if min_h <= min_v {
        let target_y = if min_v <= corner_magnet {
            snap_y
        } else {
            clamped_y
        };
        (snap_x, target_y)
    } else {
        let target_x = if min_h <= corner_magnet {
            snap_x
        } else {
            clamped_x
        };
        (target_x, snap_y)
    }
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct PipStateSnapshot {
    pub always_on_top: bool,
    pub visible_on_all_workspaces: bool,
    pub hud_hidden: bool,
    pub compact_mode: bool,
    pub snap_to_corners: bool,
    pub platform: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(default)]
pub struct PipState {
    pub always_on_top: bool,
    pub visible_on_all_workspaces: bool,
    pub hud_hidden: bool,
    pub compact_mode: bool,
    pub snap_to_corners: bool,
    pub compact_size: (f64, f64),
    pub expanded_size: (f64, f64),
    pub position: Option<(i32, i32)>,
}

impl Default for PipState {
    fn default() -> Self {
        Self {
            always_on_top: true,
            visible_on_all_workspaces: true,
            hud_hidden: true,
            compact_mode: true,
            snap_to_corners: true,
            compact_size: DEFAULT_COMPACT_SIZE,
            expanded_size: DEFAULT_EXPANDED_SIZE,
            position: None,
        }
    }
}

impl PipState {
    pub fn active_size(&self) -> (f64, f64) {
        let raw = if self.compact_mode {
            self.compact_size
        } else {
            self.expanded_size
        };
        (
            raw.0.max(MIN_WINDOW_SIZE.0),
            raw.1.max(MIN_WINDOW_SIZE.1),
        )
    }

    pub fn update_active_size(&mut self, logical_size: (f64, f64)) {
        let clamped = (
            logical_size.0.max(MIN_WINDOW_SIZE.0),
            logical_size.1.max(MIN_WINDOW_SIZE.1),
        );
        if self.compact_mode {
            self.compact_size = clamped;
        } else {
            self.expanded_size = clamped;
        }
    }

    pub fn snapshot(&self) -> PipStateSnapshot {
        PipStateSnapshot {
            always_on_top: self.always_on_top,
            visible_on_all_workspaces: self.visible_on_all_workspaces,
            hud_hidden: self.hud_hidden,
            compact_mode: self.compact_mode,
            snap_to_corners: self.snap_to_corners,
            platform: current_platform(),
        }
    }

    /// Switches between compact PiP size and expanded size, preserving the current
    /// window dimensions for whichever mode we are leaving.
    pub fn switch_compact_mode(&mut self, current_logical_size: Option<(f64, f64)>) -> (f64, f64) {
        if let Some(size) = current_logical_size {
            self.update_active_size(size);
        }
        self.compact_mode = !self.compact_mode;
        self.active_size()
    }

    pub fn reset_compact(&mut self) -> (f64, f64) {
        self.compact_size = DEFAULT_COMPACT_SIZE;
        self.compact_mode = true;
        self.compact_size
    }
}

pub fn load_preferences(path: &Path) -> PipState {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<PipState>(&raw).ok())
        .unwrap_or_default()
}

pub fn save_preferences(path: &Path, state: &PipState) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let json = serde_json::to_string_pretty(state).map_err(|err| err.to_string())?;
    let tmp_path = path.with_extension("json.tmp");
    fs::write(&tmp_path, json).map_err(|err| err.to_string())?;
    fs::rename(&tmp_path, path).map_err(|err| err.to_string())
}

/// Checks whether a saved top-left window position still intersects at least one
/// active monitor work area (so unplugging an external display won't strand the window off-screen).
pub fn position_is_visible(
    pos: (i32, i32),
    win_size: (i32, i32),
    work_areas: &[WorkAreaRect],
) -> bool {
    const MIN_VISIBLE_PX: i32 = 40;
    work_areas.iter().any(|wa| {
        let overlap_x = (pos.0 + win_size.0).min(wa.x + wa.width) - pos.0.max(wa.x);
        let overlap_y = (pos.1 + win_size.1).min(wa.y + wa.height) - pos.1.max(wa.y);
        overlap_x >= MIN_VISIBLE_PX && overlap_y >= MIN_VISIBLE_PX
    })
}

pub struct ManagedPipState {
    pub state: Mutex<PipState>,
    pub prefs_path: Mutex<Option<PathBuf>>,
}

impl ManagedPipState {
    fn new(initial: PipState) -> Self {
        Self {
            state: Mutex::new(initial),
            prefs_path: Mutex::new(None),
        }
    }

    fn persist_snapshot(&self, state_copy: &PipState) {
        let path = self.prefs_path.lock().ok().and_then(|p| p.clone());
        if let Some(path) = path {
            if let Err(err) = save_preferences(&path, state_copy) {
                eprintln!("[Norvane PiP] failed to save preferences: {err}");
            }
        }
    }
}

pub struct SnapController {
    pub move_generation: Arc<AtomicU64>,
    pub animating: Arc<AtomicBool>,
}

impl Default for SnapController {
    fn default() -> Self {
        Self {
            move_generation: Arc::new(AtomicU64::new(0)),
            animating: Arc::new(AtomicBool::new(false)),
        }
    }
}

fn is_mouse_button_pressed() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::is_mouse_button_pressed()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

pub struct MenuHandles {
    pub toggle_hud: CheckMenuItem<tauri::Wry>,
    pub always_on_top: CheckMenuItem<tauri::Wry>,
    pub all_workspaces: CheckMenuItem<tauri::Wry>,
    pub snap_to_corners: CheckMenuItem<tauri::Wry>,
    pub compact_size: CheckMenuItem<tauri::Wry>,
}

pub const fn current_platform() -> &'static str {
    if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else {
        "other"
    }
}

pub fn build_sync_script(snapshot: &PipStateSnapshot, toast: Option<&str>) -> String {
    let state_json = serde_json::to_string(snapshot).unwrap_or_else(|_| "{}".to_string());
    let toast_json = match toast {
        Some(msg) => serde_json::to_string(msg).unwrap_or_else(|_| "null".to_string()),
        None => "null".to_string(),
    };
    format!("window.__norvanePip && window.__norvanePip.syncState({state_json}, {toast_json});")
}

fn sync_menu_and_webview(app: &AppHandle, snapshot: &PipStateSnapshot, toast: Option<&str>) {
    if let Some(handles) = app.try_state::<MenuHandles>() {
        let _ = handles.toggle_hud.set_checked(snapshot.hud_hidden);
        let _ = handles.always_on_top.set_checked(snapshot.always_on_top);
        let _ = handles
            .all_workspaces
            .set_checked(snapshot.visible_on_all_workspaces);
        let _ = handles
            .snap_to_corners
            .set_checked(snapshot.snap_to_corners);
        let _ = handles.compact_size.set_checked(snapshot.compact_mode);
    }

    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
        let script = build_sync_script(snapshot, toast);
        let _ = window.eval(&script);
    }
}

fn apply_workspace_visibility_platform(
    window: &WebviewWindow,
    visible_on_all_workspaces: bool,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        macos::apply_workspace_visibility(window, visible_on_all_workspaces)
    }

    #[cfg(not(target_os = "macos"))]
    {
        window
            .set_visible_on_all_workspaces(visible_on_all_workspaces)
            .map_err(|err| err.to_string())
    }
}

fn current_logical_window_size(window: &WebviewWindow) -> Option<(f64, f64)> {
    let physical = window.inner_size().ok()?;
    let scale = window.scale_factor().ok().unwrap_or(1.0);
    let logical: LogicalSize<f64> = physical.to_logical(scale);
    Some((logical.width, logical.height))
}

fn active_work_area(window: &WebviewWindow) -> Option<(WorkAreaRect, f64)> {
    let monitor = window
        .current_monitor()
        .ok()
        .flatten()
        .or_else(|| window.primary_monitor().ok().flatten())?;
    let wa = monitor.work_area();
    let rect = WorkAreaRect {
        x: wa.position.x,
        y: wa.position.y,
        width: wa.size.width as i32,
        height: wa.size.height as i32,
    };
    Some((rect, monitor.scale_factor()))
}

fn save_current_window_geometry(app: &AppHandle, override_pos: Option<(i32, i32)>) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) else {
        return;
    };
    let pos = override_pos.or_else(|| window.outer_position().ok().map(|p| (p.x, p.y)));
    let size = current_logical_window_size(&window);

    let managed = app.state::<ManagedPipState>();
    let state_copy = {
        let Ok(mut guard) = managed.state.lock() else {
            return;
        };
        if let Some(p) = pos {
            guard.position = Some(p);
        }
        if let Some(s) = size {
            guard.update_active_size(s);
        }
        guard.clone()
    };
    managed.persist_snapshot(&state_copy);
}

fn all_monitor_work_areas(window: &WebviewWindow) -> Vec<WorkAreaRect> {
    window
        .available_monitors()
        .unwrap_or_default()
        .into_iter()
        .map(|m| {
            let wa = m.work_area();
            WorkAreaRect {
                x: wa.position.x,
                y: wa.position.y,
                width: wa.size.width as i32,
                height: wa.size.height as i32,
            }
        })
        .collect()
}

fn glide_window_to(
    app: AppHandle,
    window: &WebviewWindow,
    from: (i32, i32),
    to: (i32, i32),
    animating: Option<Arc<AtomicBool>>,
) {
    let dx = to.0 - from.0;
    let dy = to.1 - from.1;
    if dx.abs() <= 2 && dy.abs() <= 2 {
        let _ = window.set_position(Position::Physical(PhysicalPosition::new(to.0, to.1)));
        save_current_window_geometry(&app, Some(to));
        if let Some(flag) = animating {
            flag.store(false, Ordering::SeqCst);
        }
        return;
    }

    if let Some(ref flag) = animating {
        flag.store(true, Ordering::SeqCst);
    }

    let win = window.clone();
    std::thread::spawn(move || {
        const STEPS: u32 = 12;
        const STEP_DELAY: std::time::Duration = std::time::Duration::from_millis(10);
        for step in 1..=STEPS {
            let t = f64::from(step) / f64::from(STEPS);
            // Ease-out cubic glide mimicking native macOS PiP magnetic snap
            let eased = 1.0 - (1.0 - t).powi(3);
            let x = from.0 + (f64::from(dx) * eased).round() as i32;
            let y = from.1 + (f64::from(dy) * eased).round() as i32;
            let _ = win.set_position(Position::Physical(PhysicalPosition::new(x, y)));
            if step < STEPS {
                std::thread::sleep(STEP_DELAY);
            }
        }
        save_current_window_geometry(&app, Some(to));
        if let Some(flag) = animating {
            // Allow trailing OS Moved notifications from our final set_position to flush.
            std::thread::sleep(std::time::Duration::from_millis(45));
            flag.store(false, Ordering::SeqCst);
        }
    });
}

fn stick_window_to_border(
    app: &AppHandle,
    window: &WebviewWindow,
    fixed_corner: Option<ScreenCorner>,
    animate: bool,
    animating: Option<Arc<AtomicBool>>,
) {
    let Some((work_area, scale)) = active_work_area(window) else {
        return;
    };
    let Ok(outer_size) = window.outer_size() else {
        return;
    };
    let win_size = (outer_size.width as i32, outer_size.height as i32);
    let margin = (BORDER_MARGIN_LOGICAL * scale).round() as i32;
    let corner_magnet = (CORNER_MAGNET_LOGICAL * scale).round() as i32;

    let current_pos = window.outer_position().ok().map(|p| (p.x, p.y));
    let (target_x, target_y) = match fixed_corner {
        Some(corner) => corner_position(work_area, win_size, margin, corner),
        None => {
            let Some(pos) = current_pos else {
                return;
            };
            sticky_border_position(work_area, pos, win_size, margin, corner_magnet)
        }
    };

    if animate {
        if let Some(from) = current_pos {
            glide_window_to(app.clone(), window, from, (target_x, target_y), animating);
            return;
        }
    }

    if let Some(ref flag) = animating {
        flag.store(true, Ordering::SeqCst);
    }
    let _ = window.set_position(Position::Physical(PhysicalPosition::new(
        target_x, target_y,
    )));
    save_current_window_geometry(app, Some((target_x, target_y)));
    if let Some(flag) = animating {
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(45));
            flag.store(false, Ordering::SeqCst);
        });
    }
}

fn schedule_after_move_or_resize(app: &AppHandle, allow_sticky_snap: bool) {
    let controller = app.state::<SnapController>();
    if controller.animating.load(Ordering::SeqCst) {
        return;
    }

    let generation = controller.move_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let move_gen = Arc::clone(&controller.move_generation);
    let animating = Arc::clone(&controller.animating);
    let app_handle = app.clone();

    std::thread::spawn(move || {
        // Initial short settle delay; on non-macOS use slightly longer debounce.
        let initial_delay = if cfg!(target_os = "macos") { 70 } else { 180 };
        std::thread::sleep(std::time::Duration::from_millis(initial_delay));

        // On macOS, wait until the user actually releases the mouse button so we never fight a drag.
        while move_gen.load(Ordering::SeqCst) == generation && is_mouse_button_pressed() {
            std::thread::sleep(std::time::Duration::from_millis(35));
        }

        if move_gen.load(Ordering::SeqCst) != generation || animating.load(Ordering::SeqCst) {
            return;
        }

        let should_snap = allow_sticky_snap
            && app_handle
                .state::<ManagedPipState>()
                .state
                .lock()
                .map(|g| g.compact_mode && g.snap_to_corners)
                .unwrap_or(false);

        if should_snap {
            if let Some(window) = app_handle.get_webview_window(MAIN_WINDOW_LABEL) {
                stick_window_to_border(&app_handle, &window, None, true, Some(animating));
            }
        } else {
            save_current_window_geometry(&app_handle, None);
        }
    });
}

fn apply_always_on_top(
    app: &AppHandle,
    enabled: bool,
    toast: Option<&str>,
) -> Result<PipStateSnapshot, String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main window not found".to_string())?;
    window
        .set_always_on_top(enabled)
        .map_err(|err| err.to_string())?;

    let managed = app.state::<ManagedPipState>();
    let (snapshot, state_copy) = {
        let mut guard = managed.state.lock().map_err(|_| "State lock poisoned")?;
        guard.always_on_top = enabled;
        (guard.snapshot(), guard.clone())
    };
    managed.persist_snapshot(&state_copy);

    sync_menu_and_webview(app, &snapshot, toast);
    Ok(snapshot)
}

fn apply_all_workspaces(
    app: &AppHandle,
    enabled: bool,
    toast: Option<&str>,
) -> Result<PipStateSnapshot, String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main window not found".to_string())?;
    apply_workspace_visibility_platform(&window, enabled)?;

    let managed = app.state::<ManagedPipState>();
    let (snapshot, state_copy) = {
        let mut guard = managed.state.lock().map_err(|_| "State lock poisoned")?;
        guard.visible_on_all_workspaces = enabled;
        (guard.snapshot(), guard.clone())
    };
    managed.persist_snapshot(&state_copy);

    sync_menu_and_webview(app, &snapshot, toast);
    Ok(snapshot)
}

fn apply_snap_to_corners(
    app: &AppHandle,
    enabled: bool,
    toast: Option<&str>,
) -> Result<PipStateSnapshot, String> {
    let managed = app.state::<ManagedPipState>();
    let (snapshot, state_copy) = {
        let mut guard = managed.state.lock().map_err(|_| "State lock poisoned")?;
        guard.snap_to_corners = enabled;
        (guard.snapshot(), guard.clone())
    };
    managed.persist_snapshot(&state_copy);

    if enabled && snapshot.compact_mode {
        if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
            let animating = Arc::clone(&app.state::<SnapController>().animating);
            stick_window_to_border(app, &window, None, true, Some(animating));
        }
    }

    sync_menu_and_webview(app, &snapshot, toast);
    Ok(snapshot)
}

fn apply_hud_hidden(
    app: &AppHandle,
    hidden: bool,
    toast: Option<&str>,
) -> Result<PipStateSnapshot, String> {
    let managed = app.state::<ManagedPipState>();
    let (snapshot, state_copy) = {
        let mut guard = managed.state.lock().map_err(|_| "State lock poisoned")?;
        guard.hud_hidden = hidden;
        (guard.snapshot(), guard.clone())
    };
    managed.persist_snapshot(&state_copy);

    sync_menu_and_webview(app, &snapshot, toast);
    Ok(snapshot)
}

fn apply_toggle_compact_size(app: &AppHandle, toast: bool) -> Result<PipStateSnapshot, String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main window not found".to_string())?;
    let current = current_logical_window_size(&window);

    let managed = app.state::<ManagedPipState>();
    let (target_size, snapshot, state_copy) = {
        let mut guard = managed.state.lock().map_err(|_| "State lock poisoned")?;
        let target = guard.switch_compact_mode(current);
        (target, guard.snapshot(), guard.clone())
    };
    managed.persist_snapshot(&state_copy);

    window
        .set_size(Size::Logical(LogicalSize::new(
            target_size.0,
            target_size.1,
        )))
        .map_err(|err| err.to_string())?;

    if snapshot.compact_mode && snapshot.snap_to_corners {
        let animating = Arc::clone(&app.state::<SnapController>().animating);
        stick_window_to_border(app, &window, None, true, Some(animating));
    } else if !snapshot.compact_mode {
        let _ = window.center();
        save_current_window_geometry(app, None);
    }

    let msg = if toast {
        Some(if snapshot.compact_mode {
            "Compact PiP Size"
        } else {
            "Expanded Size"
        })
    } else {
        None
    };

    sync_menu_and_webview(app, &snapshot, msg);
    Ok(snapshot)
}

fn apply_reset_compact_size(app: &AppHandle) -> Result<PipStateSnapshot, String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main window not found".to_string())?;

    let managed = app.state::<ManagedPipState>();
    let (target_size, snapshot, state_copy) = {
        let mut guard = managed.state.lock().map_err(|_| "State lock poisoned")?;
        let target = guard.reset_compact();
        (target, guard.snapshot(), guard.clone())
    };
    managed.persist_snapshot(&state_copy);

    window
        .set_size(Size::Logical(LogicalSize::new(
            target_size.0,
            target_size.1,
        )))
        .map_err(|err| err.to_string())?;

    if snapshot.snap_to_corners {
        let animating = Arc::clone(&app.state::<SnapController>().animating);
        stick_window_to_border(app, &window, Some(ScreenCorner::BottomRight), true, Some(animating));
    }

    sync_menu_and_webview(app, &snapshot, Some("Reset PiP (440×275)"));
    Ok(snapshot)
}

#[tauri::command]
fn get_pip_state(state: State<'_, ManagedPipState>) -> Result<PipStateSnapshot, String> {
    let guard = state.state.lock().map_err(|_| "State lock poisoned")?;
    Ok(guard.snapshot())
}

#[tauri::command]
fn set_hud_hidden(app: AppHandle, hidden: bool) -> Result<PipStateSnapshot, String> {
    apply_hud_hidden(&app, hidden, None)
}

#[tauri::command]
fn toggle_always_on_top(
    app: AppHandle,
    state: State<'_, ManagedPipState>,
) -> Result<PipStateSnapshot, String> {
    let next = {
        let guard = state.state.lock().map_err(|_| "State lock poisoned")?;
        !guard.always_on_top
    };
    apply_always_on_top(&app, next, None)
}

#[tauri::command]
fn toggle_all_workspaces(
    app: AppHandle,
    state: State<'_, ManagedPipState>,
) -> Result<PipStateSnapshot, String> {
    let next = {
        let guard = state.state.lock().map_err(|_| "State lock poisoned")?;
        !guard.visible_on_all_workspaces
    };
    apply_all_workspaces(&app, next, None)
}

#[tauri::command]
fn toggle_compact_size(app: AppHandle) -> Result<PipStateSnapshot, String> {
    apply_toggle_compact_size(&app, false)
}

#[tauri::command]
fn start_pip_drag(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main window not found".to_string())?;
    window.start_dragging().map_err(|err| err.to_string())
}

#[tauri::command]
fn close_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main window not found".to_string())?;
    window.close().map_err(|err| err.to_string())
}

#[tauri::command]
fn minimize_window(app: AppHandle) -> Result<(), String> {
    let window = app
        .get_webview_window(MAIN_WINDOW_LABEL)
        .ok_or_else(|| "Main window not found".to_string())?;
    window.minimize().map_err(|err| err.to_string())
}

fn build_app_menu(
    app: &AppHandle,
    initial: &PipStateSnapshot,
) -> tauri::Result<(Menu<tauri::Wry>, MenuHandles)> {
    let toggle_hud = CheckMenuItem::with_id(
        app,
        MENU_TOGGLE_HUD,
        "Hide Game HUD",
        true,
        initial.hud_hidden,
        Some("CmdOrCtrl+Shift+H"),
    )?;
    let always_on_top = CheckMenuItem::with_id(
        app,
        MENU_ALWAYS_ON_TOP,
        "Always on Top",
        true,
        initial.always_on_top,
        Some("CmdOrCtrl+Shift+T"),
    )?;
    let all_workspaces = CheckMenuItem::with_id(
        app,
        MENU_ALL_WORKSPACES,
        "Visible on All Workspaces",
        true,
        initial.visible_on_all_workspaces,
        Some("CmdOrCtrl+Shift+W"),
    )?;
    let snap_to_corners = CheckMenuItem::with_id(
        app,
        MENU_STICKY_BORDERS,
        "Sticky to Screen Borders",
        true,
        initial.snap_to_corners,
        Some("CmdOrCtrl+Shift+S"),
    )?;
    let compact_size = CheckMenuItem::with_id(
        app,
        MENU_COMPACT_SIZE,
        "Compact PiP Size",
        true,
        initial.compact_mode,
        Some("CmdOrCtrl+Shift+P"),
    )?;
    let reset_compact_size = MenuItem::with_id(
        app,
        MENU_RESET_COMPACT_SIZE,
        "Reset Compact PiP (440×275)",
        true,
        None::<&str>,
    )?;
    let reload_game = MenuItem::with_id(
        app,
        MENU_RELOAD_GAME,
        "Reload Game",
        true,
        Some("CmdOrCtrl+R"),
    )?;
    let shortcuts = MenuItem::with_id(
        app,
        MENU_SHORTCUTS,
        "Keyboard Shortcuts",
        true,
        Some("CmdOrCtrl+Shift+/"),
    )?;

    let view_menu = Submenu::with_items(
        app,
        "View",
        true,
        &[
            &toggle_hud,
            &always_on_top,
            &all_workspaces,
            &snap_to_corners,
            &PredefinedMenuItem::separator(app)?,
            &compact_size,
            &reset_compact_size,
            &PredefinedMenuItem::separator(app)?,
            &reload_game,
            &shortcuts,
        ],
    )?;

    let edit_menu = Submenu::with_items(
        app,
        "Edit",
        true,
        &[
            &PredefinedMenuItem::undo(app, None)?,
            &PredefinedMenuItem::redo(app, None)?,
            &PredefinedMenuItem::separator(app)?,
            &PredefinedMenuItem::cut(app, None)?,
            &PredefinedMenuItem::copy(app, None)?,
            &PredefinedMenuItem::paste(app, None)?,
            &PredefinedMenuItem::select_all(app, None)?,
        ],
    )?;

    let window_menu = Submenu::with_items(
        app,
        "Window",
        true,
        &[
            &PredefinedMenuItem::minimize(app, None)?,
            &PredefinedMenuItem::close_window(app, None)?,
        ],
    )?;

    #[cfg(target_os = "macos")]
    let menu = {
        let app_menu = Submenu::with_items(
            app,
            "Norvane PiP",
            true,
            &[
                &PredefinedMenuItem::about(app, None, None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::hide(app, None)?,
                &PredefinedMenuItem::hide_others(app, None)?,
                &PredefinedMenuItem::show_all(app, None)?,
                &PredefinedMenuItem::separator(app)?,
                &PredefinedMenuItem::quit(app, None)?,
            ],
        )?;
        Menu::with_items(app, &[&app_menu, &edit_menu, &view_menu, &window_menu])?
    };

    #[cfg(not(target_os = "macos"))]
    let menu = {
        let file_menu = Submenu::with_items(
            app,
            "File",
            true,
            &[&PredefinedMenuItem::quit(app, None)?],
        )?;
        Menu::with_items(app, &[&file_menu, &edit_menu, &view_menu, &window_menu])?
    };

    let handles = MenuHandles {
        toggle_hud,
        always_on_top,
        all_workspaces,
        snap_to_corners,
        compact_size,
    };

    Ok((menu, handles))
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(ManagedPipState::new(PipState::default()))
        .manage(SnapController::default())
        .invoke_handler(tauri::generate_handler![
            get_pip_state,
            set_hud_hidden,
            toggle_always_on_top,
            toggle_all_workspaces,
            toggle_compact_size,
            start_pip_drag,
            close_window,
            minimize_window,
        ])
        .setup(|app| {
            let prefs_path = app
                .path()
                .app_config_dir()
                .ok()
                .map(|dir| dir.join(PREFS_FILE_NAME));
            let loaded_state = prefs_path
                .as_deref()
                .map(load_preferences)
                .unwrap_or_default();

            {
                let managed = app.state::<ManagedPipState>();
                if let Ok(mut guard) = managed.state.lock() {
                    *guard = loaded_state.clone();
                }
                if let Ok(mut path_guard) = managed.prefs_path.lock() {
                    *path_guard = prefs_path;
                };
            }

            let initial_snapshot = loaded_state.snapshot();
            let initial_size = loaded_state.active_size();

            let (menu, handles) = build_app_menu(app.handle(), &initial_snapshot)?;
            app.set_menu(menu)?;
            app.manage(handles);

            let target_url = GAME_URL
                .parse()
                .expect("GAME_URL must be a valid external URL");

            let builder = WebviewWindowBuilder::new(
                app,
                MAIN_WINDOW_LABEL,
                WebviewUrl::External(target_url),
            )
            .title("Norvane PiP")
            .inner_size(initial_size.0, initial_size.1)
            .min_inner_size(MIN_WINDOW_SIZE.0, MIN_WINDOW_SIZE.1)
            .resizable(true)
            .shadow(true)
            .always_on_top(initial_snapshot.always_on_top)
            .visible_on_all_workspaces(initial_snapshot.visible_on_all_workspaces)
            .initialization_script(PIP_OVERLAY_SCRIPT);

            #[cfg(target_os = "macos")]
            let builder = builder
                .title_bar_style(tauri::TitleBarStyle::Overlay)
                .hidden_title(true);

            #[cfg(not(target_os = "macos"))]
            let builder = builder.decorations(false);

            let window = builder.build()?;

            #[cfg(target_os = "macos")]
            if let Err(err) = macos::configure_native_pip_window(&window) {
                eprintln!("[Norvane PiP] macOS native PiP chrome warning: {err}");
            }

            if let Err(err) = apply_workspace_visibility_platform(
                &window,
                initial_snapshot.visible_on_all_workspaces,
            ) {
                eprintln!("[Norvane PiP] workspace visibility setup warning: {err}");
            }

            let animating = Arc::clone(&app.state::<SnapController>().animating);
            let mut restored_position = false;
            if let Some(saved_pos) = loaded_state.position {
                let win_size = window
                    .outer_size()
                    .map(|s| (s.width as i32, s.height as i32))
                    .unwrap_or((initial_size.0 as i32, initial_size.1 as i32));
                let work_areas = all_monitor_work_areas(&window);
                if position_is_visible(saved_pos, win_size, &work_areas) {
                    animating.store(true, Ordering::SeqCst);
                    let _ = window.set_position(Position::Physical(PhysicalPosition::new(
                        saved_pos.0,
                        saved_pos.1,
                    )));
                    let flag = Arc::clone(&animating);
                    std::thread::spawn(move || {
                        std::thread::sleep(std::time::Duration::from_millis(60));
                        flag.store(false, Ordering::SeqCst);
                    });
                    restored_position = true;
                }
            }

            if !restored_position && initial_snapshot.compact_mode && initial_snapshot.snap_to_corners {
                stick_window_to_border(
                    app.handle(),
                    &window,
                    Some(ScreenCorner::BottomRight),
                    false,
                    Some(animating),
                );
            }

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() != MAIN_WINDOW_LABEL {
                return;
            }
            match event {
                WindowEvent::Moved(_) => {
                    schedule_after_move_or_resize(window.app_handle(), true);
                }
                WindowEvent::Resized(_) => {
                    schedule_after_move_or_resize(window.app_handle(), false);
                }
                WindowEvent::CloseRequested { .. } => {
                    save_current_window_geometry(window.app_handle(), None);
                }
                _ => {}
            }
        })
        .on_menu_event(|app, event| {
            let id = event.id().as_ref();
            match id {
                MENU_TOGGLE_HUD => {
                    let next = app
                        .state::<ManagedPipState>()
                        .state
                        .lock()
                        .map(|g| !g.hud_hidden)
                        .unwrap_or(false);
                    let msg = if next { "HUD Hidden" } else { "HUD Visible" };
                    let _ = apply_hud_hidden(app, next, Some(msg));
                }
                MENU_ALWAYS_ON_TOP => {
                    let next = app
                        .state::<ManagedPipState>()
                        .state
                        .lock()
                        .map(|g| !g.always_on_top)
                        .unwrap_or(true);
                    let msg = if next {
                        "Always on Top: On"
                    } else {
                        "Always on Top: Off"
                    };
                    let _ = apply_always_on_top(app, next, Some(msg));
                }
                MENU_ALL_WORKSPACES => {
                    let next = app
                        .state::<ManagedPipState>()
                        .state
                        .lock()
                        .map(|g| !g.visible_on_all_workspaces)
                        .unwrap_or(true);
                    let msg = if next {
                        "All Workspaces: On"
                    } else {
                        "All Workspaces: Off"
                    };
                    let _ = apply_all_workspaces(app, next, Some(msg));
                }
                MENU_STICKY_BORDERS => {
                    let next = app
                        .state::<ManagedPipState>()
                        .state
                        .lock()
                        .map(|g| !g.snap_to_corners)
                        .unwrap_or(true);
                    let msg = if next {
                        "Sticky Borders: On"
                    } else {
                        "Sticky Borders: Off"
                    };
                    let _ = apply_snap_to_corners(app, next, Some(msg));
                }
                MENU_COMPACT_SIZE => {
                    let _ = apply_toggle_compact_size(app, true);
                }
                MENU_RESET_COMPACT_SIZE => {
                    let _ = apply_reset_compact_size(app);
                }
                MENU_RELOAD_GAME => {
                    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                        let _ = window.eval("window.location.reload();");
                    }
                }
                MENU_SHORTCUTS => {
                    if let Some(window) = app.get_webview_window(MAIN_WINDOW_LABEL) {
                        let _ = window.eval(
                            "window.__norvanePip && window.__norvanePip.toggleShortcuts();",
                        );
                    }
                }
                _ => {}
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Norvane PiP");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_state_matches_native_pip_targets() {
        let state = PipState::default();
        assert!(state.always_on_top);
        assert!(state.visible_on_all_workspaces);
        assert!(state.hud_hidden);
        assert!(state.compact_mode);
        assert!(state.snap_to_corners);
        assert_eq!(state.compact_size, DEFAULT_COMPACT_SIZE);
        assert_eq!(state.expanded_size, DEFAULT_EXPANDED_SIZE);
    }

    #[test]
    fn switching_compact_mode_preserves_resized_dimensions() {
        let mut state = PipState::default();

        let expanded = state.switch_compact_mode(Some((520.0, 340.0)));
        assert!(!state.compact_mode);
        assert_eq!(expanded, DEFAULT_EXPANDED_SIZE);
        assert_eq!(state.compact_size, (520.0, 340.0));

        let compact = state.switch_compact_mode(Some((1280.0, 800.0)));
        assert!(state.compact_mode);
        assert_eq!(compact, (520.0, 340.0));
        assert_eq!(state.expanded_size, (1280.0, 800.0));
    }

    #[test]
    fn reset_compact_restores_default_size_and_mode() {
        let mut state = PipState::default();
        state.switch_compact_mode(Some((600.0, 400.0)));
        assert!(!state.compact_mode);

        let size = state.reset_compact();
        assert!(state.compact_mode);
        assert_eq!(size, DEFAULT_COMPACT_SIZE);
    }

    #[test]
    fn sticky_border_snaps_to_nearest_edge_and_locks_corners() {
        let work_area = WorkAreaRect {
            x: 0,
            y: 38,
            width: 1920,
            height: 1042,
        };
        let win_size = (440, 275);
        let margin = 16;
        let magnet = 48;

        // Mid-right drop sticks to right border while preserving Y
        let mid_right = sticky_border_position(work_area, (1320, 400), win_size, margin, magnet);
        assert_eq!(mid_right, (1920 - 440 - 16, 400));

        // Mid-left drop sticks to left border while preserving Y
        let mid_left = sticky_border_position(work_area, (120, 420), win_size, margin, magnet);
        assert_eq!(mid_left, (16, 420));

        // Top-center drop sticks to top border while preserving X
        let top_center = sticky_border_position(work_area, (700, 70), win_size, margin, magnet);
        assert_eq!(top_center, (700, 38 + 16));

        // Near bottom-right locks cleanly into bottom-right corner
        let br = sticky_border_position(work_area, (1420, 760), win_size, margin, magnet);
        assert_eq!(br, (1920 - 440 - 16, 38 + 1042 - 275 - 16));
    }

    #[test]
    fn target_url_is_valid_https() {
        let parsed: tauri::Url = GAME_URL.parse().expect("GAME_URL should parse");
        assert_eq!(parsed.scheme(), "https");
        assert_eq!(parsed.host_str(), Some("ro-idle.pages.dev"));
    }

    #[test]
    fn sync_script_serializes_snapshot_and_toast_safely() {
        let snapshot = PipState::default().snapshot();
        let script = build_sync_script(&snapshot, Some("HUD \"Hidden\""));
        assert!(script.contains("window.__norvanePip.syncState("));
        assert!(script.contains("\"always_on_top\":true"));
        assert!(script.contains("\"snap_to_corners\":true"));
        assert!(script.contains("\"HUD \\\"Hidden\\\"\""));
    }

    #[test]
    fn preferences_round_trip_preserves_toggles_sizes_and_position() {
        let dir = std::env::temp_dir().join(format!("norvane-pip-test-{}", std::process::id()));
        let path = dir.join("preferences.json");
        let saved = PipState {
            always_on_top: false,
            visible_on_all_workspaces: false,
            hud_hidden: false,
            compact_mode: false,
            snap_to_corners: false,
            compact_size: (500.0, 310.0),
            expanded_size: (1240.0, 780.0),
            position: Some((1440, 680)),
        };

        save_preferences(&path, &saved).expect("save should succeed");
        let loaded = load_preferences(&path);
        assert_eq!(loaded, saved);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn position_visibility_checks_monitor_overlap() {
        let monitors = [WorkAreaRect {
            x: 0,
            y: 38,
            width: 1920,
            height: 1042,
        }];
        assert!(position_is_visible((1400, 700), (440, 275), &monitors));
        assert!(!position_is_visible((2500, 100), (440, 275), &monitors));
    }
}
