use std::{collections::BTreeMap, fs, path::PathBuf};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};

use crate::companion::{
    error::{CompanionError, Result},
    protocol::CompanionCredentials,
};

const STATE_FILE_NAME: &str = "companion_state.json";

#[derive(Debug, Default, Serialize, Deserialize)]
struct StoredState {
    profiles: BTreeMap<String, CompanionCredentials>,
}

#[derive(Clone, Debug)]
pub struct CredentialStore {
    path: PathBuf,
}

impl CredentialStore {
    pub fn for_app(app: &AppHandle) -> Result<Self> {
        let mut path = app
            .path()
            .app_data_dir()
            .map_err(|_| CompanionError::AppDataPathUnavailable)?;
        path.push(STATE_FILE_NAME);
        Ok(Self { path })
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    pub fn get_credentials(&self, profile: &str) -> Result<Option<CompanionCredentials>> {
        Ok(self.load()?.profiles.get(profile).cloned())
    }

    pub fn put_credentials(&self, profile: &str, credentials: CompanionCredentials) -> Result<()> {
        let mut state = self.load()?;
        state.profiles.insert(profile.to_string(), credentials);
        self.save(&state)
    }

    fn load(&self) -> Result<StoredState> {
        if !self.path.exists() {
            return Ok(StoredState::default());
        }
        let raw = fs::read_to_string(&self.path)?;
        Ok(serde_json::from_str(&raw)?)
    }

    fn save(&self, state: &StoredState) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        }
        let encoded = serde_json::to_string_pretty(state)?;
        fs::write(&self.path, format!("{encoded}\n"))?;
        Ok(())
    }
}
