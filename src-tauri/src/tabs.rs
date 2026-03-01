use serde::{Deserialize, Serialize};
use std::sync::Mutex;

/// Serializable snapshot sent to the frontend via IPC.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TabInfo {
    pub id: u32,
    pub url: String,
    pub title: String,
    pub favicon_url: Option<String>,
    pub is_active: bool,
}

/// Full internal representation.
#[derive(Debug)]
pub struct Tab {
    pub id: u32,
    /// Tauri webview label, e.g. "tab-1".
    pub webview_label: String,
    pub url: String,
    pub title: String,
    pub favicon_url: Option<String>,
}

impl Tab {
    pub fn new(id: u32, url: impl Into<String>) -> Self {
        let url = url.into();
        Tab {
            id,
            webview_label: format!("tab-{}", id),
            url: url.clone(),
            title: url,
            favicon_url: None,
        }
    }

    pub fn to_info(&self, is_active: bool) -> TabInfo {
        TabInfo {
            id: self.id,
            url: self.url.clone(),
            title: self.title.clone(),
            favicon_url: self.favicon_url.clone(),
            is_active,
        }
    }
}

#[derive(Default, Debug)]
pub struct TabManager {
    pub tabs: Vec<Tab>,
    pub active_id: Option<u32>,
    next_id: u32,
}

impl TabManager {
    pub fn add_tab(&mut self, url: impl Into<String>) -> &Tab {
        self.next_id += 1;
        let tab = Tab::new(self.next_id, url);
        self.active_id = Some(tab.id);
        self.tabs.push(tab);
        self.tabs.last().unwrap()
    }

    pub fn remove_tab(&mut self, id: u32) {
        if let Some(pos) = self.tabs.iter().position(|t| t.id == id) {
            self.tabs.remove(pos);
            if self.active_id == Some(id) {
                self.active_id = self.tabs.get(pos.saturating_sub(1)).map(|t| t.id);
            }
        }
    }

    pub fn set_active(&mut self, id: u32) {
        if self.tabs.iter().any(|t| t.id == id) {
            self.active_id = Some(id);
        }
    }

    pub fn get_tab_mut(&mut self, id: u32) -> Option<&mut Tab> {
        self.tabs.iter_mut().find(|t| t.id == id)
    }

    pub fn get_tab(&self, id: u32) -> Option<&Tab> {
        self.tabs.iter().find(|t| t.id == id)
    }

    pub fn all_infos(&self) -> Vec<TabInfo> {
        self.tabs
            .iter()
            .map(|t| t.to_info(Some(t.id) == self.active_id))
            .collect()
    }
}

/// Mutex-wrapped state registered with Tauri's `manage()`.
pub type TabState = Mutex<TabManager>;
