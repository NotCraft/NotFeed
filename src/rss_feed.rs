use crate::config::Config;
use chrono::{Date, DateTime, Duration, Utc};
use reqwest::Client;
use rss::Channel;
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::fmt::{Debug, Display};
use std::fs::File;
use tracing::{info, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyRss {
    #[serde(with = "date_format")]
    pub(crate) date: Date<Utc>,
    pub(crate) channels: Option<Vec<Channel>>,
}

#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Rss {
    pub(crate) site_title: Option<String>,
    pub(crate) build_time: Option<DateTime<Utc>>,
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
            date: Utc::today(),
            channels: Some(channels),
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

        let mut rss: Rss = if let Some(cache_url) = &config.cache_url {
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

        let today = Utc::today();
        let cache_day = today - Duration::days(config.cache_max_days);

        rss.site_title = Some(config.site_title.clone());
        rss.days = rss
            .days
            .into_iter()
            .filter(|d| d.date > cache_day && d.date != today)
            .collect();

        info!("Feeding today's Rss!");
        rss.days
            .push(DailyRss::new(&config.sources, &client).await?);

        let skip = vec![
            "itunes_ext",
            "dublin_core_ext",
            "syndication_ext",
            "namespaces",
            "categories",
            "extensions",
            "skip_hours",
            "skip_days",
        ]
        .into_iter()
        .map(|x| x.to_string())
        .collect();

        let cache = clean_json(json!(&rss), &skip);

        let mut f = File::create("target/cache.json")?;
        serde_json::to_writer(&mut f, &cache)?;

        Ok(rss)
    }
}

fn clean_json(value: Value, skip: &HashSet<String>) -> Value {
    match value {
        Value::Null => Value::Null,
        Value::Bool(b) => Value::Bool(b),
        Value::Number(n) => Value::Number(n),
        Value::String(s) => Value::String(s),
        Value::Array(a) => {
            if a.is_empty() {
                Value::Null
            } else {
                let res: Vec<Value> = a
                    .into_iter()
                    .map(|x| clean_json(x, skip))
                    .filter(|v| !v.is_null())
                    .collect();
                Value::Array(res)
            }
        }
        Value::Object(o) => {
            if o.is_empty() {
                Value::Null
            } else {
                let res: Map<String, Value> = o
                    .into_iter()
                    .map(|(k, v)| {
                        if skip.contains(&k) {
                            (k, v)
                        } else {
                            (k, clean_json(v, skip))
                        }
                    })
                    .filter(|(k, v)| skip.contains(k) || !v.is_null())
                    .collect();
                if res.is_empty() {
                    Value::Null
                } else {
                    Value::Object(res)
                }
            }
        }
    }
}

async fn feed_cache<T: reqwest::IntoUrl + Display>(
    url: T,
    client: &Client,
) -> Result<Rss, Box<dyn std::error::Error>> {
    Ok(client.get(url).send().await?.json().await?)
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

    fn feed_cache_local() -> Result<Rss, Box<dyn std::error::Error>> {
        use std::io::BufReader;
        let file = File::open("target/cache.json")?;
        let reader = BufReader::new(file);
        Ok(serde_json::from_reader(reader)?)
    }

    #[tokio::test]
    async fn feed_cache_load() -> Result<(), Box<dyn std::error::Error>> {
        let mut config = Config::default();
        config
            .sources
            .push("http://export.arxiv.org/rss/cs.CL".to_string());
        let _rss = Rss::feed_rss(&config).await?;

        let _feed = feed_cache_local()?;
        Ok(())
    }
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
