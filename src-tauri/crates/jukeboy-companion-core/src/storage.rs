use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};

use crate::{error::Result, protocol::CompanionCredentials};

pub const STATE_FILE_NAME: &str = "companion_state.json";

#[derive(Debug, Default, Serialize, Deserialize)]
struct StoredState {
    profiles: BTreeMap<String, CompanionCredentials>,
}

#[derive(Clone, Debug)]
pub struct CredentialStore {
    path: PathBuf,
}

impl CredentialStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn for_app_data_dir(app_data_dir: impl AsRef<Path>) -> Self {
        let mut path = app_data_dir.as_ref().to_path_buf();
        path.push(STATE_FILE_NAME);
        Self { path }
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

impl Default for CredentialStore {
    fn default() -> Self {
        Self::new(STATE_FILE_NAME)
    }
}
