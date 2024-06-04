use core::fmt;
use std::{collections::HashMap, path::PathBuf, sync::Arc};

use serde::{
    de::{self, Visitor},
    Deserialize, Deserializer, Serialize, Serializer,
};
use tokio::sync::{RwLock, RwLockReadGuard};

use crate::PlayerAccount;

pub const STATE_DIR: &str = "somnustate/";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(transparent)]
pub struct PlayerId(
    #[serde(
        serialize_with = "serialize_u128_hex",
        deserialize_with = "deserialize_u128_hex"
    )]
    u128,
);

/// Keeps track of player accounts, linking a username and password
/// to an easily copied ID.
#[derive(Debug, Clone)]
pub struct AccountStorage(Arc<RwLock<HashMap<PlayerId, PlayerAccount>>>, PathBuf);

impl AccountStorage {
    pub async fn load_or_new(filename: &str) -> anyhow::Result<Self> {
        let path = make_save_path(filename);

        let values = if tokio::fs::try_exists(&path).await.is_ok_and(|r| r) {
            let yaml = tokio::fs::read_to_string(&path).await?;
            serde_yaml::from_str(&yaml)?
        } else {
            if let Some(dir) = path.parent() {
                tokio::fs::create_dir_all(dir).await?;
            }

            HashMap::new()
        };

        Ok(AccountStorage(RwLock::new(values).into(), path))
    }

    pub async fn register_user(&self, player: PlayerAccount) -> anyhow::Result<PlayerId> {
        let mut write = self.0.write().await;
        let id = PlayerId(rand::random());
        write.insert(id, player);

        let yaml = serde_yaml::to_string::<HashMap<PlayerId, PlayerAccount>>(&write)?;
        tokio::fs::write(&self.1, yaml).await?;

        Ok(id)
    }

    pub async fn read(&self) -> RwLockReadGuard<HashMap<PlayerId, PlayerAccount>> {
        self.0.read().await
    }

    pub fn blocking_read(&self) -> RwLockReadGuard<HashMap<PlayerId, PlayerAccount>> {
        self.0.blocking_read()
    }
}

/// We use u128 for some IDs, we save them as a hex string which
/// is a little less efficient but looks much better.
struct HexIdVisitor;

impl<'de> Visitor<'de> for HexIdVisitor {
    type Value = u128;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("A hex string representing a u128 integer")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        Ok(u128::from_str_radix(v, 16)
            .map_err(|e| E::custom(format!("could not parse hex string: {e}")))?)
    }
}

pub fn serialize_u128_hex<S>(v: &u128, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(&format!("{v:x}"))
}

pub fn deserialize_u128_hex<'de, D>(deserializer: D) -> Result<u128, D::Error>
where
    D: Deserializer<'de>,
{
    deserializer.deserialize_str(HexIdVisitor)
}

pub fn make_save_path(filename: &str) -> PathBuf {
    let mut path: PathBuf = STATE_DIR.into();
    path.push(filename);
    path
}
