use figment::{
    error::Result,
    providers::{Format, Serialized},
    providers::{Toml, Yaml},
    Figment,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::info;

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub(crate) cache_max_days: i64,
    pub(crate) cache_url: String,
    pub(crate) templates_dir: String,
    pub(crate) statics_dir: String,
    pub(crate) site_title: String,
    pub(crate) proxy: Option<String>,
    pub(crate) sources: Vec<String>,
    pub(crate) scripts: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            cache_url: "".to_string(),
            site_title: "".to_string(),
            templates_dir: "includes".to_string(),
            statics_dir: "statics".to_string(),
            cache_max_days: 0,
            proxy: Default::default(),
            sources: Default::default(),
            scripts: Default::default(),
        }
    }
}

impl Config {
    pub fn new() -> Result<Config> {
        info!("Loading config!");
        Figment::from(Serialized::defaults(Config::default()))
            .merge(Yaml::file("Config.yaml"))
            .merge(Toml::file("Config.toml"))
            .extract()
    }
}
