use crate::config::Config;
use chrono::{Date, DateTime, Duration, Utc};
use reqwest::blocking::Client;
use rss::Channel;
use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};
use std::fs::File;
use tracing::info;

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
    pub fn new<'t, T: reqwest::IntoUrl + ToString + Clone + Debug + Display>(
        urls: &'t [T],
        client: &Client,
    ) -> Result<DailyRss, Box<dyn std::error::Error>>
    where
        &'t T: 't + Display,
    {
        let mut channels = vec![];
        for url in urls {
            info!("Feeding rss from {}", url);
            channels.push({
                let content = client.get(url.clone()).send()?.bytes()?;
                let mut channel = Channel::read_from(&content[..])?;
                channel.link = url.to_string();
                channel
            });
        }

        Ok(DailyRss {
            date: Utc::today(),
            channels,
        })
    }
}

impl Rss {
    pub fn feed_rss(config: &Config) -> Result<Rss, Box<dyn std::error::Error>> {
        info!("Building rss Client!");
        let client = match &config.proxy {
            None => reqwest::blocking::Client::builder().build()?,
            Some(scheme) => reqwest::blocking::Client::builder()
                .proxy(reqwest::Proxy::all(scheme)?)
                .build()?,
        };

        info!("Feeding Rss Cache!");

        let mut rss: Rss = match feed_cache(&config.cache_url, &client) {
            Ok(rss) => rss,
            Err(_err) => Default::default(),
        };

        let today = Utc::today();
        let cache_day = today - Duration::days(config.cache_max_days);

        rss.days = rss
            .days
            .into_iter()
            .filter(|d| d.date > cache_day && d.date != today)
            .collect();

        info!("Feeding today's Rss!");
        rss.days.push(DailyRss::new(&config.sources, &client)?);

        let mut f = File::create("target/cache.json")?;
        serde_json::to_writer(&mut f, &rss)?;

        Ok(rss)
    }
}

#[cfg(not(debug_assertions))]
fn feed_cache<T: reqwest::IntoUrl>(
    url: T,
    client: &Client,
) -> Result<Rss, Box<dyn std::error::Error>> {
    Ok(client.get(url).send()?.json()?)
}

#[cfg(debug_assertions)]
fn feed_cache<T: reqwest::IntoUrl + Debug>(
    _url: T,
    _client: &Client,
) -> Result<Rss, Box<dyn std::error::Error>> {
    use std::io::BufReader;
    let file = File::open("target/cache.json")?;
    let reader = BufReader::new(file);
    Ok(serde_json::from_reader(reader)?)
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
