mod config;
mod render;
mod rhai_regex;
mod rss_feed;

use clap::{AppSettings, Clap};
use config::Config;
use fs_extra::dir::{copy, CopyOptions};
use render::handlebars;
use rss_feed::Rss;
use std::fs;
use std::fs::File;
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use tracing::{info, span};
use warp::{self, Filter};

#[derive(Clap)]
#[clap(version = "v0.1.1", author = "Feng Yunlong <ylfeng@ir.hit.edu.cn>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    #[clap(
        author = "Feng Yunlong <ylfeng@ir.hit.edu.cn>",
        about = "Server serve."
    )]
    Serve(Serve),

    #[clap(author = "Feng Yunlong <ylfeng@ir.hit.edu.cn>", about = "Build.")]
    Build,
}

#[derive(Clap)]
struct Serve {
    #[clap(short, long, default_value = "127.0.0.1", about = "addr export")]
    addr: String,
    #[clap(short, long, default_value = "8080", about = "port export")]
    port: u16,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    tracing_subscriber::fmt()
        .with_target(false)
        .try_init()
        .expect("Tracing init error!");
    let root = span!(tracing::Level::INFO, "<FEED>");
    let _enter = root.enter();

    let config = Config::new()?;

    info!("Copying static files!");
    fs::create_dir_all("target")?;
    if Path::new(&config.statics_dir).exists() {
        let mut options = CopyOptions::new();
        options.content_only = true;
        options.overwrite = true;
        copy(&config.statics_dir, "target", &options)?;
    }

    let rss = Rss::feed_rss(&config).await?;
    let hbs = handlebars(&config)?;

    match opts.subcmd {
        SubCommand::Serve(serve) => {
            let hbs_ref = Arc::new(hbs);
            let socks: SocketAddr = format!("{}:{}", serve.addr, serve.port).parse()?;

            let route = warp::get().and(warp::path::end()).map(move || {
                let result = hbs_ref
                    .render("index", &rss)
                    .unwrap_or_else(|e| e.to_string());
                warp::reply::html(result)
            });
            let static_files = warp::fs::dir(&config.statics_dir);

            // GET / => rendered index templates
            // GET /... => statics_dir/...
            let routes = route.or(static_files);

            warp::serve(routes).run(socks).await;
        }
        SubCommand::Build => {
            let mut output_file = File::create("target/index.html")?;
            hbs.render_to_write("index", &rss, &mut output_file)?;
            println!("target/index.html generated");
        }
    }

    Ok(())
}
