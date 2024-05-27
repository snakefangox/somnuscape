use std::{path::PathBuf, sync::Arc};

use serde::{Deserialize, Serialize};
use tokio::sync::{RwLock, RwLockReadGuard};

pub const STATE_DIR: &str = "somnustate/";

/// A storage mechanism for global data that can be set but never changed,
/// Like player account info
#[derive(Debug, Clone)]
pub struct Registry<T>(Arc<RwLock<Vec<T>>>, PathBuf);

impl<T: for<'a> Deserialize<'a> + Serialize> Registry<T> {
    pub async fn load_or_new(filename: &str) -> anyhow::Result<Self> {
        let mut path: PathBuf = STATE_DIR.into();
        path.push(filename);

        let values = if tokio::fs::try_exists(&path).await.is_ok_and(|r| r) {
            let yaml = tokio::fs::read_to_string(&path).await?;
            serde_yaml::from_str(&yaml)?
        } else {
            if let Some(dir) = path.parent() {
                tokio::fs::create_dir_all(dir).await?;
            }
            Vec::new()
        };

        Ok(Registry(RwLock::new(values).into(), path))
    }

    pub async fn add_user(&self, player: T) -> anyhow::Result<usize> {
        let mut write = self.0.write().await;
        let id = write.len();
        write.push(player);

        let yaml = serde_yaml::to_string::<Vec<T>>(write.as_ref())?;
        tokio::fs::write(&self.1, yaml).await?;

        Ok(id)
    }

    pub async fn read(&self) -> RwLockReadGuard<Vec<T>> {
        self.0.read().await
    }

    pub fn blocking_read(&self) -> RwLockReadGuard<Vec<T>> {
        self.0.blocking_read()
    }
}
