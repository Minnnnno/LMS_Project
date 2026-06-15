use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

const VIEWER_TIMEOUT: Duration = Duration::from_secs(45);

#[derive(Clone, Default)]
pub struct AppState {
    viewers: Arc<Mutex<HashMap<String, Instant>>>,
}

impl AppState {
    pub fn record_viewer(&self, viewer_id: String) -> usize {
        let now = Instant::now();
        let mut viewers = self.viewers.lock().unwrap_or_else(|error| error.into_inner());

        viewers.retain(|_, last_seen| now.duration_since(*last_seen) <= VIEWER_TIMEOUT);
        viewers.insert(viewer_id, now);
        viewers.len()
    }

    pub fn active_viewers(&self) -> usize {
        let now = Instant::now();
        let mut viewers = self.viewers.lock().unwrap_or_else(|error| error.into_inner());

        viewers.retain(|_, last_seen| now.duration_since(*last_seen) <= VIEWER_TIMEOUT);
        viewers.len()
    }

    pub fn remove_viewer(&self, viewer_id: &str) -> usize {
        let mut viewers = self.viewers.lock().unwrap_or_else(|error| error.into_inner());

        viewers.remove(viewer_id);
        viewers.len()
    }
}
