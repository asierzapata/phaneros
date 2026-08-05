use tauri::{
  menu::{Menu, MenuItem},
  tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
  Manager, WindowEvent,
};

mod autostart;
mod commands;
mod conflicts;
mod daemon_locate;
mod format;
mod fs_scan;
mod ipc_client;

fn toggle_tray_window(app: &tauri::AppHandle, tray_rect: Option<tauri::Rect>) {
  let Some(window) = app.get_webview_window("tray") else {
    return;
  };

  if window.is_visible().unwrap_or(false) {
    let _ = window.hide();
    return;
  }

  if let (Some(rect), Ok(win_size), Ok(scale_factor)) =
    (tray_rect, window.outer_size(), window.scale_factor())
  {
    let icon_pos = rect.position.to_physical::<i32>(scale_factor);
    let icon_size = rect.size.to_physical::<i32>(scale_factor);
    let x = icon_pos.x + icon_size.width / 2 - win_size.width as i32 / 2;
    let y = icon_pos.y + icon_size.height;
    let _ = window.set_position(tauri::PhysicalPosition::new(x, y));
  }

  let _ = window.show();
  let _ = window.set_focus();
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(tauri_plugin_opener::init())
    .plugin(tauri_plugin_dialog::init())
    .invoke_handler(tauri::generate_handler![
      commands::list_vaults,
      commands::get_telemetry,
      commands::list_activity,
      commands::trigger_sync,
      commands::get_file_tree,
      commands::list_conflicts,
      commands::get_conflict_diff,
      commands::resolve_conflict,
      commands::daemon_ping,
      commands::start_daemon,
      commands::add_vault,
      commands::load_onboarding_state,
      commands::save_onboarding_state,
      autostart::register_login_item,
      autostart::unregister_login_item,
      autostart::is_login_item_registered,
    ])
    .setup(|app| {
      let show_item = MenuItem::with_id(app, "show", "Open Phaneros", true, None::<&str>)?;
      let quit_item = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
      let tray_menu = Menu::with_items(app, &[&show_item, &quit_item])?;

      TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&tray_menu)
        .show_menu_on_left_click(false)
        .tooltip("Phaneros")
        .on_menu_event(|app, event| match event.id.as_ref() {
          "show" => {
            if let Some(window) = app.get_webview_window("main") {
              let _ = window.show();
              let _ = window.set_focus();
            }
          }
          "quit" => {
            app.exit(0);
          }
          _ => {}
        })
        .on_tray_icon_event(|tray, event| {
          if let TrayIconEvent::Click {
            button: MouseButton::Left,
            button_state: MouseButtonState::Up,
            rect,
            ..
          } = event
          {
            toggle_tray_window(tray.app_handle(), Some(rect));
          }
        })
        .build(app)?;

      Ok(())
    })
    .on_window_event(|window, event| {
      // Keep the app alive in the tray instead of quitting when the main
      // window or the tray popup is closed.
      if let WindowEvent::CloseRequested { api, .. } = event {
        let _ = window.hide();
        api.prevent_close();
      }
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
