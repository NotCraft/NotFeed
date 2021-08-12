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
    pub(crate) minify: bool,
    pub(crate) cache_max_days: i64,
    pub(crate) site_title: String,
    pub(crate) target_dir: String,
    pub(crate) statics_dir: String,
    pub(crate) templates_dir: String,
    pub(crate) proxy: Option<String>,
    pub(crate) cache_url: Option<String>,
    pub(crate) target_name: Option<String>,
    pub(crate) sources: Vec<String>,
    pub(crate) scripts: HashMap<String, String>,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            minify: false,
            cache_max_days: 0,
            site_title: crate_name!().to_string(),
            target_dir: "target".to_string(),
            statics_dir: "statics".to_string(),
            templates_dir: "includes".to_string(),
            proxy: None,
            cache_url: None,
            target_name: None,
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
