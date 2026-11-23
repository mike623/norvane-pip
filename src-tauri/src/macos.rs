use objc2_app_kit::{
    NSEvent, NSWindow, NSWindowButton, NSWindowCollectionBehavior, NSWindowTitleVisibility,
};
use tauri::WebviewWindow;

/// Returns `true` if any mouse button is currently pressed on macOS.
/// Used to avoid snapping a PiP window while the user is still holding a drag.
pub fn is_mouse_button_pressed() -> bool {
    NSEvent::pressedMouseButtons() != 0
}

/// Computes the target `NSWindowCollectionBehavior` for macOS workspace visibility.
///
/// AppKit treats `CanJoinAllSpaces` and `MoveToActiveSpace` as mutually exclusive
/// and throws `NSInvalidArgumentException` if both bits are set simultaneously.
/// Adding `FullScreenAuxiliary` also allows the floating PiP window to appear over
/// full-screen Spaces.
pub fn compute_collection_behavior(
    current: NSWindowCollectionBehavior,
    visible_on_all_workspaces: bool,
) -> NSWindowCollectionBehavior {
    let mut next = current;
    if visible_on_all_workspaces {
        next.remove(NSWindowCollectionBehavior::MoveToActiveSpace);
        next.remove(NSWindowCollectionBehavior::FullScreenPrimary);
        next.insert(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    } else {
        next.remove(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::FullScreenAuxiliary,
        );
    }
    next
}

/// Configures the macOS `NSWindow` to mimic a native Picture-in-Picture window:
/// preserves native rounded corners, drop shadow, and edge resizing while hiding
/// the titlebar chrome and standard traffic-light buttons.
pub fn configure_native_pip_window(window: &WebviewWindow) -> Result<(), String> {
    let win_handle = window.clone();
    window
        .run_on_main_thread(move || {
            if let Ok(raw_ptr) = win_handle.ns_window() {
                if !raw_ptr.is_null() {
                    // SAFETY: We are on the macOS main thread and Tauri guarantees `ns_window()`
                    // returns a valid `NSWindow*` for the lifetime of this window.
                    let ns_window = unsafe { &*(raw_ptr as *const NSWindow) };
                    ns_window.setTitlebarAppearsTransparent(true);
                    ns_window.setTitleVisibility(NSWindowTitleVisibility::Hidden);
                    ns_window.setHasShadow(true);
                    ns_window.setMovableByWindowBackground(false);

                    for button_kind in [
                        NSWindowButton::CloseButton,
                        NSWindowButton::MiniaturizeButton,
                        NSWindowButton::ZoomButton,
                    ] {
                        if let Some(btn) = ns_window.standardWindowButton(button_kind) {
                            btn.setHidden(true);
                        }
                    }
                }
            }
        })
        .map_err(|err| err.to_string())
}

pub fn apply_workspace_visibility(
    window: &WebviewWindow,
    visible_on_all_workspaces: bool,
) -> Result<(), String> {
    window
        .set_visible_on_all_workspaces(visible_on_all_workspaces)
        .map_err(|err| err.to_string())?;

    let win_handle = window.clone();
    window
        .run_on_main_thread(move || {
            if let Ok(raw_ptr) = win_handle.ns_window() {
                if !raw_ptr.is_null() {
                    // SAFETY: We are on the macOS main thread and Tauri guarantees `ns_window()`
                    // returns a valid `NSWindow*` for the lifetime of this window.
                    let ns_window = unsafe { &*(raw_ptr as *const NSWindow) };
                    let current = ns_window.collectionBehavior();
                    let next = compute_collection_behavior(current, visible_on_all_workspaces);
                    ns_window.setCollectionBehavior(next);
                }
            }
        })
        .map_err(|err| err.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enables_all_spaces_and_fullscreen_auxiliary_without_conflicting_bits() {
        let initial = NSWindowCollectionBehavior::MoveToActiveSpace
            | NSWindowCollectionBehavior::FullScreenPrimary;
        let next = compute_collection_behavior(initial, true);

        assert!(next.contains(NSWindowCollectionBehavior::CanJoinAllSpaces));
        assert!(next.contains(NSWindowCollectionBehavior::FullScreenAuxiliary));
        assert!(!next.contains(NSWindowCollectionBehavior::MoveToActiveSpace));
        assert!(!next.contains(NSWindowCollectionBehavior::FullScreenPrimary));
    }

    #[test]
    fn disables_all_spaces_and_fullscreen_auxiliary_cleanly() {
        let initial = NSWindowCollectionBehavior::CanJoinAllSpaces
            | NSWindowCollectionBehavior::FullScreenAuxiliary
            | NSWindowCollectionBehavior::Stationary;
        let next = compute_collection_behavior(initial, false);

        assert!(!next.contains(NSWindowCollectionBehavior::CanJoinAllSpaces));
        assert!(!next.contains(NSWindowCollectionBehavior::FullScreenAuxiliary));
        assert!(next.contains(NSWindowCollectionBehavior::Stationary));
    }
}
