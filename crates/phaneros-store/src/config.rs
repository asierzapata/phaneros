use std::{net::IpAddr, path::PathBuf};

use figment::{
    Figment,
    providers::{Env, Serialized},
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub host: IpAddr,
    pub port: u16,
    pub database_path: PathBuf,
    pub blob_storage_path: PathBuf,
    pub public_url: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            host: IpAddr::from([127, 0, 0, 1]),
            port: 3000,
            database_path: PathBuf::from("phaneros.db"),
            blob_storage_path: PathBuf::from("blobs"),
            public_url: "http://127.0.0.1:3000".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, figment::Error> {
        Figment::new()
            .merge(Serialized::defaults(Config::default()))
            .merge(Env::raw())
            .extract()
    }
}
