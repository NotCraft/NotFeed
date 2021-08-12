use crate::config::Config;
use chrono::{DateTime, Duration, Utc};
use clap::crate_version;
use reqwest::Client;
use rss::Channel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::fs;
use std::fs::File;
use std::path::Path;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyRss {
    pub(crate) datetime: DateTime<Utc>,
    pub(crate) channels: Vec<Channel>,
}

impl Default for DailyRss {
    fn default() -> DailyRss {
        DailyRss {
            datetime: Utc::now(),
            channels: vec![],
        }
    }
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Rss {
    pub(crate) site_title: String,
    pub(crate) project_name: String,
    pub(crate) project_version: String,
    pub(crate) project_homepage: String,
    pub(crate) days: Vec<DailyRss>,
}

impl DailyRss {
    pub async fn new<'t, T: reqwest::IntoUrl + ToString + Clone + Debug + Display>(
        urls: &'t [T],
        client: &Client,
    ) -> Result<DailyRss, Box<dyn std::error::Error>>
    where
        &'t T: 't + Display,
    {
        let mut channels = vec![];
        for url in urls {
            info!("Feeding rss from {}", url);
            let response = client.get(url.clone()).send().await;
            match response {
                Ok(content) => {
                    let content = content.bytes().await?;
                    let mut channel = Channel::read_from(&content[..])?;
                    channel.link = url.to_string();
                    channels.push(channel);
                }
                Err(err) => {
                    warn!("Failed: {}!", err.to_string());
                }
            };
        }

        Ok(DailyRss {
            datetime: Utc::now(),
            channels,
        })
    }
}

impl Rss {
    pub async fn feed_rss(config: &Config) -> Result<Rss, Box<dyn std::error::Error>> {
        info!("Building rss client!");
        let client = match &config.proxy {
            None => reqwest::Client::builder().build()?,
            Some(scheme) => reqwest::Client::builder()
                .proxy(reqwest::Proxy::all(scheme)?)
                .build()?,
        };

        let rss: Rss = if let Some(cache_url) = &config.cache_url {
            info!("Feeding rss cache from {}", cache_url);
            match feed_cache(cache_url, &client).await {
                Ok(rss) => {
                    info!("Feed rss cache Successfully!");
                    rss
                }
                Err(err) => {
                    warn!("Failed: {}!", err.to_string());
                    Default::default()
                }
            }
        } else {
            Default::default()
        };

        let mut rss_items = rss.days;
        info!("Feeding today's Rss!");
        rss_items.push(DailyRss::new(&config.sources, &client).await?);
        let mut rss_days: HashMap<DateTime<Utc>, Vec<Channel>> = HashMap::new();

        for day in rss_items {
            for channel in day.channels {
                let date = match &channel.dublin_core_ext {
                    None => day.datetime,
                    Some(ext) => (&ext.dates[0]).parse()?,
                };
                let entry = rss_days.entry(date).or_default();
                entry.push(channel);
            }
        }

        let today = Utc::today();
        let cache_day = today - Duration::days(config.cache_max_days);
        let cache_day = cache_day.and_hms(0, 0, 0);

        let rss_days = rss_days
            .into_iter()
            .map(|(datetime, mut channels)| {
                channels.sort_by_key(|c| c.link.to_owned());
                channels.dedup();
                DailyRss { datetime, channels }
            })
            .filter(|d| d.datetime > cache_day)
            .collect();

        let mut rss = Rss {
            site_title: config.site_title.clone(),
            project_name: crate_name!().to_string(),
            project_version: crate_version!().to_string(),
            project_homepage: crate_homepage!().to_string(),
            days: rss_days,
        };

        rss.days.sort_by(|a, b| b.datetime.cmp(&a.datetime));
        fs::create_dir_all(&config.target_dir)?;
        let cache_path = Path::new(&config.target_dir).join("cache.json");
        let mut f = File::create(cache_path)?;
        serde_json::to_writer(&mut f, &rss)?;

        Ok(rss)
    }
}

async fn feed_cache<T: reqwest::IntoUrl + Display>(
    url: T,
    client: &Client,
) -> Result<Rss, Box<dyn std::error::Error>> {
    Ok(client.get(url).send().await?.json().await?)
}
