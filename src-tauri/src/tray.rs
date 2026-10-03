//! Tray / menu-bar icon: connection, last check-in, waiting jobs, Quit.
//!
//! Menu labels start in English and are replaced by the UI language as soon as
//! the webview sends them.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use synaplan_core::poll::PollStatus;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Manager, Wry};

static QUITTING: AtomicBool = AtomicBool::new(false);

pub fn is_quitting() -> bool {
    QUITTING.load(Ordering::SeqCst)
}

pub fn begin_quit() {
    QUITTING.store(true, Ordering::SeqCst);
}

#[derive(Clone)]
struct TrayCopy {
    connected: String,
    not_connected: String,
    last_checkin: String,
    no_checkin: String,
    no_jobs: String,
    jobs_waiting: String,
    quit: String,
}

impl Default for TrayCopy {
    fn default() -> Self {
        Self {
            connected: "Connected".into(),
            not_connected: "Not connected".into(),
            last_checkin: "Last check-in recorded".into(),
            no_checkin: "No check-in yet".into(),
            no_jobs: "No jobs waiting".into(),
            jobs_waiting: "{count} jobs waiting".into(),
            quit: "Quit".into(),
        }
    }
}

static TRAY_COPY: Mutex<TrayCopy> = Mutex::new(TrayCopy {
    connected: String::new(),
    not_connected: String::new(),
    last_checkin: String::new(),
    no_checkin: String::new(),
    no_jobs: String::new(),
    jobs_waiting: String::new(),
    quit: String::new(),
});

/// `"{count} job waiting | {count} jobs waiting"` — the same split vue-i18n uses.
fn plural_count(template: &str, count: u32) -> String {
    let chosen = match template.split(" | ").collect::<Vec<_>>().as_slice() {
        [one, many] => {
            if count == 1 {
                *one
            } else {
                *many
            }
        }
        _ => template,
    };
    chosen.replace("{count}", &count.to_string())
}

fn tray_copy() -> TrayCopy {
    let guard = TRAY_COPY.lock().unwrap_or_else(|e| e.into_inner());
    if guard.quit.is_empty() {
        TrayCopy::default()
    } else {
        guard.clone()
    }
}

pub struct TrayLabels {
    pub connected: String,
    pub not_connected: String,
    pub last_checkin: String,
    pub no_checkin: String,
    pub no_jobs: String,
    pub jobs_waiting: String,
    pub quit: String,
}

pub fn set_labels(labels: TrayLabels) {
    let mut guard = TRAY_COPY.lock().unwrap_or_else(|e| e.into_inner());
    *guard = TrayCopy {
        connected: labels.connected,
        not_connected: labels.not_connected,
        last_checkin: labels.last_checkin,
        no_checkin: labels.no_checkin,
        no_jobs: labels.no_jobs,
        jobs_waiting: labels.jobs_waiting,
        quit: labels.quit,
    };
}

pub struct TrayHandles {
    pub connected: MenuItem<Wry>,
    pub last: MenuItem<Wry>,
    pub jobs: MenuItem<Wry>,
    pub quit: MenuItem<Wry>,
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
        quit,
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
                begin_quit();
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
    let copy = tray_copy();
    let connected = if paired {
        copy.connected
    } else {
        copy.not_connected
    };
    let last = if poll.last_checkin_unix.is_some() {
        copy.last_checkin
    } else {
        copy.no_checkin
    };
    let jobs = if poll.jobs_waiting == 0 {
        copy.no_jobs
    } else {
        plural_count(&copy.jobs_waiting, poll.jobs_waiting)
    };
    let _ = handles.connected.set_text(connected);
    let _ = handles.last.set_text(last);
    let _ = handles.jobs.set_text(jobs);
    let _ = handles.quit.set_text(copy.quit);
}
