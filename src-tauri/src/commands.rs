use crate::tabs::{TabInfo, TabState};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};

#[cfg(desktop)]
use tauri::WebviewUrl;

// ---------------------------------------------------------------------------
// Shared payload types
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ContentBounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Debug, Serialize)]
struct TabTitleChangedPayload {
    tab_id: u32,
    title: String,
}

#[derive(Clone, Debug, Serialize)]
struct NavigationCommittedPayload {
    tab_id: u32,
    url: String,
}

// ---------------------------------------------------------------------------
// Child webview creation — desktop only (mobile has no child webview API)
// ---------------------------------------------------------------------------

#[cfg(desktop)]
async fn spawn_tab_webview(
    app: &AppHandle,
    label: &str,
    url: &str,
    tab_id: u32,
    bounds: &ContentBounds,
) {
    let window = match app.get_window("main") {
        Some(w) => w,
        None => return,
    };

    let parsed_url = match url.parse::<tauri::Url>() {
        Ok(u) => u,
        Err(e) => {
            eprintln!("Invalid URL '{url}': {e}");
            return;
        }
    };

    let app_clone = app.clone();
    let tab_id_copy = tab_id;

    // Identify as a modern Chrome browser so sites like Google serve their
    // current UI instead of a legacy fallback they send to unknown WebKit UAs.
    let ua = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) \
              AppleWebKit/537.36 (KHTML, like Gecko) \
              Chrome/125.0.0.0 Safari/537.36";

    let webview_builder = tauri::webview::WebviewBuilder::new(
        label,
        WebviewUrl::External(parsed_url),
    )
    .user_agent(ua)
    .on_document_title_changed({
        let app = app_clone.clone();
        move |_webview, title| {
            {
                let state: State<TabState> = app.state();
                let mut manager = state.lock().unwrap();
                if let Some(tab) = manager.get_tab_mut(tab_id_copy) {
                    tab.title = title.clone();
                }
            }
            app.emit(
                "tab-title-changed",
                TabTitleChangedPayload { tab_id: tab_id_copy, title },
            )
            .ok();
        }
    })
    .on_navigation({
        let app = app_clone.clone();
        move |url| {
            let url_str = url.to_string();
            {
                let state: State<TabState> = app.state();
                let mut manager = state.lock().unwrap();
                if let Some(tab) = manager.get_tab_mut(tab_id_copy) {
                    tab.url = url_str.clone();
                }
            }
            app.emit(
                "navigation-committed",
                NavigationCommittedPayload { tab_id: tab_id_copy, url: url_str },
            )
            .ok();
            true
        }
    });

    let position = tauri::LogicalPosition::new(bounds.x, bounds.y);
    let size = tauri::LogicalSize::new(bounds.width, bounds.height);

    let is_active = {
        let state: State<TabState> = app.state();
        let manager = state.lock().unwrap();
        manager.active_id == Some(tab_id)
    };

    match window.add_child(webview_builder, position, size) {
        Ok(webview) => {
            if !is_active {
                webview.hide().ok();
            }
        }
        Err(e) => eprintln!("Failed to create child webview for tab {tab_id}: {e}"),
    }
}

// Mobile stub — tab state is still tracked, but no child webview is created.
#[cfg(not(desktop))]
async fn spawn_tab_webview(
    _app: &AppHandle,
    _label: &str,
    _url: &str,
    _tab_id: u32,
    _bounds: &ContentBounds,
) {
}

// ---------------------------------------------------------------------------
// IPC Commands
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn get_tabs(state: State<TabState>) -> Vec<TabInfo> {
    state.lock().unwrap().all_infos()
}

#[tauri::command]
pub async fn new_tab(
    app: AppHandle,
    state: State<'_, TabState>,
    url: Option<String>,
    bounds: ContentBounds,
) -> Result<Vec<TabInfo>, String> {
    let url = url.unwrap_or_else(|| "https://start.duckduckgo.com".to_string());

    let (label, id) = {
        let mut manager = state.lock().unwrap();
        let tab = manager.add_tab(&url);
        (tab.webview_label.clone(), tab.id)
    };

    #[cfg(desktop)]
    hide_all_tab_webviews(&app, &state, id);

    spawn_tab_webview(&app, &label, &url, id, &bounds).await;

    let tabs = state.lock().unwrap().all_infos();
    app.emit("tabs-updated", tabs.clone()).unwrap();
    Ok(tabs)
}

#[tauri::command]
pub async fn close_tab(
    app: AppHandle,
    state: State<'_, TabState>,
    tab_id: u32,
) -> Result<Vec<TabInfo>, String> {
    // Validate tab exists
    {
        let manager = state.lock().unwrap();
        if manager.get_tab(tab_id).is_none() {
            return Err(format!("Tab {tab_id} not found"));
        }
    }

    // Destroy the child webview on desktop
    #[cfg(desktop)]
    {
        let label = state
            .lock()
            .unwrap()
            .get_tab(tab_id)
            .map(|t| t.webview_label.clone())
            .unwrap();
        if let Some(webview) = app.get_webview(&label) {
            webview.close().ok();
        }
    }

    {
        let mut manager = state.lock().unwrap();
        manager.remove_tab(tab_id);
    }

    let tabs = state.lock().unwrap().all_infos();

    // Reveal the newly-active webview on desktop
    #[cfg(desktop)]
    {
        let active_id = state.lock().unwrap().active_id;
        if let Some(active_id) = active_id {
            let active_label = state
                .lock()
                .unwrap()
                .get_tab(active_id)
                .map(|t| t.webview_label.clone());
            if let Some(lbl) = active_label {
                if let Some(webview) = app.get_webview(&lbl) {
                    webview.show().ok();
                    webview.set_focus().ok();
                }
            }
        }
    }

    app.emit("tabs-updated", tabs.clone()).unwrap();
    Ok(tabs)
}

#[tauri::command]
pub fn switch_tab(
    app: AppHandle,
    state: State<TabState>,
    tab_id: u32,
) -> Result<Vec<TabInfo>, String> {
    #[cfg(desktop)]
    hide_all_tab_webviews(&app, &state, tab_id);

    {
        let mut manager = state.lock().unwrap();
        if manager.get_tab(tab_id).is_none() {
            return Err(format!("Tab {tab_id} not found"));
        }
        manager.set_active(tab_id);
    }

    #[cfg(desktop)]
    {
        let label = state
            .lock()
            .unwrap()
            .get_tab(tab_id)
            .map(|t| t.webview_label.clone())
            .unwrap();
        if let Some(webview) = app.get_webview(&label) {
            webview.show().ok();
            webview.set_focus().ok();
        }
    }

    let tabs = state.lock().unwrap().all_infos();
    app.emit("tabs-updated", tabs.clone()).unwrap();
    Ok(tabs)
}

#[tauri::command]
pub fn navigate(
    app: AppHandle,
    state: State<TabState>,
    url: String,
) -> Result<(), String> {
    let label = {
        let manager = state.lock().unwrap();
        let active_id = manager.active_id.ok_or("No active tab")?;
        manager
            .get_tab(active_id)
            .map(|t| t.webview_label.clone())
            .ok_or_else(|| "Active tab not found".to_string())?
    };

    if let Some(webview) = app.get_webview(&label) {
        let parsed = url.parse().map_err(|e| format!("Invalid URL: {e}"))?;
        webview.navigate(parsed).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn go_back(app: AppHandle, state: State<TabState>) -> Result<(), String> {
    with_active_webview(&app, &state, |wv| {
        wv.eval("history.back()").ok();
    })
}

#[tauri::command]
pub fn go_forward(app: AppHandle, state: State<TabState>) -> Result<(), String> {
    with_active_webview(&app, &state, |wv| {
        wv.eval("history.forward()").ok();
    })
}

#[cfg_attr(not(desktop), allow(unused_variables))]
#[tauri::command]
pub fn update_content_bounds(
    app: AppHandle,
    state: State<TabState>,
    bounds: ContentBounds,
) -> Result<(), String> {
    #[cfg(desktop)]
    {
        let labels: Vec<String> = state
            .lock()
            .unwrap()
            .tabs
            .iter()
            .map(|t| t.webview_label.clone())
            .collect();

        for label in labels {
            if let Some(webview) = app.get_webview(&label) {
                webview
                    .set_bounds(tauri::Rect {
                        position: tauri::LogicalPosition::new(bounds.x, bounds.y).into(),
                        size: tauri::LogicalSize::new(bounds.width, bounds.height).into(),
                    })
                    .ok();
            }
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

#[cfg(desktop)]
fn hide_all_tab_webviews(app: &AppHandle, state: &State<TabState>, except_id: u32) {
    let labels: Vec<(u32, String)> = state
        .lock()
        .unwrap()
        .tabs
        .iter()
        .map(|t| (t.id, t.webview_label.clone()))
        .collect();

    for (id, label) in labels {
        if id != except_id {
            if let Some(webview) = app.get_webview(&label) {
                webview.hide().ok();
            }
        }
    }
}

fn with_active_webview<F>(
    app: &AppHandle,
    state: &State<TabState>,
    f: F,
) -> Result<(), String>
where
    F: FnOnce(&tauri::Webview),
{
    let label = {
        let manager = state.lock().unwrap();
        let active_id = manager.active_id.ok_or("No active tab")?;
        manager
            .get_tab(active_id)
            .map(|t| t.webview_label.clone())
            .ok_or_else(|| "Active tab not found".to_string())?
    };

    if let Some(webview) = app.get_webview(&label) {
        f(&webview);
    }
    Ok(())
}
