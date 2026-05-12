#[cfg(target_os = "android")]
pub use jukeboy_companion_core::android_ble;
pub use jukeboy_companion_core::{client, error, mock, protocol, service, storage};
pub use service::*;

use tauri::{AppHandle, Emitter, Manager};

use jukeboy_companion_core::{
    error::{CompanionError, Result},
    protocol::EVENT_NAME,
    storage::CredentialStore,
    CompanionManager as CoreCompanionManager,
};

pub struct AppState {
    manager: CoreCompanionManager,
}

impl AppState {
    pub fn for_app(app: &AppHandle) -> Result<Self> {
        let app_data_dir = app
            .path()
            .app_data_dir()
            .map_err(|_| CompanionError::AppDataPathUnavailable)?;

        Ok(Self::new(CredentialStore::for_app_data_dir(app_data_dir)))
    }

    pub fn new(store: CredentialStore) -> Self {
        Self {
            manager: CoreCompanionManager::new(store),
        }
    }

    pub fn manager(&self) -> &CoreCompanionManager {
        &self.manager
    }

    pub fn spawn_event_bridge(&self, app: AppHandle) {
        let mut event_rx = self.manager.subscribe_events();
        tauri::async_runtime::spawn(async move {
            loop {
                match event_rx.recv().await {
                    Ok(event) => {
                        let _ = app.emit(EVENT_NAME, event);
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
        });
    }
}
