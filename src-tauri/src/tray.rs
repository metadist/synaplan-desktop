//! Tray / menu-bar icon: connection, last check-in, waiting jobs, Quit.

use synaplan_core::poll::PollStatus;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

pub struct TrayHandles {
    pub connected: MenuItem<Wry>,
    pub last: MenuItem<Wry>,
    pub jobs: MenuItem<Wry>,
}

pub fn setup(app: &tauri::App) -> tauri::Result<()> {
    let connected = MenuItem::with_id(app, "status", "Not connected", false, None::<&str>)?;
    let last = MenuItem::with_id(app, "last", "No check-in yet", false, None::<&str>)?;
    let jobs = MenuItem::with_id(app, "jobs", "No jobs waiting", false, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&connected, &last, &jobs, &quit])?;

    app.manage(TrayHandles {
        connected,
        last,
        jobs,
    });

    let icon = app
        .default_window_icon()
        .cloned()
        .expect("app icon is bundled");

    TrayIconBuilder::with_id("main")
        .icon(icon)
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            if event.id() == "quit" {
                app.exit(0);
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                if let Some(window) = tray.app_handle().get_webview_window("main") {
                    let _ = window.unminimize();
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        })
        .build(app)?;
    Ok(())
}

pub fn refresh(app: &AppHandle, poll: &PollStatus) {
    let Some(handles) = app.try_state::<TrayHandles>() else {
        return;
    };
    let paired = crate::commands::status_of(&app.state::<crate::commands::AppState>())
        .map(|s| s.paired)
        .unwrap_or(false);
    let connected = if paired { "Connected" } else { "Not connected" };
    let last = if poll.last_checkin_unix.is_some() {
        "Last check-in recorded"
    } else {
        "No check-in yet"
    };
    let jobs = if poll.jobs_waiting == 0 {
        "No jobs waiting".into()
    } else {
        format!("{} jobs waiting", poll.jobs_waiting)
    };
    let _ = handles.connected.set_text(connected);
    let _ = handles.last.set_text(last);
    let _ = handles.jobs.set_text(jobs);
}
