fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new().app_manifest(
            tauri_build::AppManifest::new().commands(&[
                "get_pip_state",
                "set_hud_hidden",
                "toggle_always_on_top",
                "toggle_all_workspaces",
                "toggle_compact_size",
                "start_pip_drag",
                "close_window",
                "minimize_window",
            ]),
        ),
    )
    .expect("failed to run tauri-build");
}
