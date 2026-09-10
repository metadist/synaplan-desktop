//! Microphone access for dictation.
//!
//! The webview asks for the microphone through `getUserMedia`. Windows
//! (WebView2) and macOS (WKWebView, with the `NSMicrophoneUsageDescription`
//! in `Info.plist`) show their own permission prompt. WebKitGTK on Linux has
//! no prompt UI and *denies* an unhandled `permission-request` outright, so
//! without this hook every Linux install would fail dictation with
//! `NotAllowedError`.
//!
//! The webview only ever loads this app's own UI and a take starts with the
//! user's click on the mic button, so that click is the consent: an
//! audio-only request is granted here. Anything else (camera, location,
//! notifications, display capture) keeps the platform default, which is deny.

use tauri::AppHandle;

pub fn setup(app: &AppHandle) {
    #[cfg(target_os = "linux")]
    {
        use tauri::Manager;
        if let Some(window) = app.get_webview_window("main") {
            let _ = window.with_webview(|webview| {
                use webkit2gtk::glib::Cast;
                use webkit2gtk::{
                    PermissionRequestExt, UserMediaPermissionRequest,
                    UserMediaPermissionRequestExt, WebViewExt,
                };
                webview.inner().connect_permission_request(|_, request| {
                    let Some(media) = request.downcast_ref::<UserMediaPermissionRequest>() else {
                        return false;
                    };
                    if media.is_for_audio_device() && !media.is_for_video_device() {
                        request.allow();
                    } else {
                        request.deny();
                    }
                    true
                });
            });
        }
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = app;
    }
}
