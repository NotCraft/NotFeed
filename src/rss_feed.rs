use crate::config::Config;
use chrono::{Date, DateTime, Duration, Utc};
use reqwest::Client;
use rss::Channel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::fs::File;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyRss {
    #[serde(with = "date_format")]
    pub(crate) date: Date<Utc>,
    pub(crate) channels: Vec<Channel>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rss {
    pub(crate) site_title: String,
    pub(crate) build_time: DateTime<Utc>,
    pub(crate) days: Vec<DailyRss>,
}

impl Default for Rss {
    fn default() -> Rss {
        Rss {
            site_title: "".to_string(),
            build_time: Utc::now(),
            days: vec![],
        }
    }
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
            date: Utc::today(),
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
                    None => day.date.and_hms(0, 0, 0),
                    Some(ext) => (&ext.dates[0]).parse()?,
                };
                let entry = rss_days.entry(date).or_default();
                entry.push(channel);
            }
        }

        let today = Utc::today();
        let cache_day = today - Duration::days(config.cache_max_days);

        let rss_days = rss_days
            .into_iter()
            .map(|(time, mut channels)| {
                channels.dedup();
                DailyRss {
                    date: time.date(),
                    channels,
                }
            })
            .filter(|d| d.date > cache_day && d.date != today)
            .collect();

        let mut rss = Rss {
            site_title: config.site_title.clone(),
            build_time: Utc::now(),
            days: rss_days,
        };

        rss.days.sort_by(|a, b| b.date.cmp(&a.date));
        let mut f = File::create("target/cache.json")?;
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

mod date_format {
    use chrono::{Date, NaiveDate, Utc};
    use serde::{self, Deserialize, Deserializer, Serializer};

    const FORMAT: &str = "%Y-%m-%d";

    pub fn serialize<S>(date: &Date<Utc>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let s = format!("{}", date.format(FORMAT));
        serializer.serialize_str(&s)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Date<Utc>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        NaiveDate::parse_from_str(&s, FORMAT)
            .map(|d| Date::from_utc(d, Utc))
            .map_err(serde::de::Error::custom)
    }
}
