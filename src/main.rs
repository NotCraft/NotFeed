#[macro_use]
mod utils;
mod config;
mod render;
mod rhai_regex;
mod rss_feed;

use crate::utils::copy_statics_to_target;
use clap::{crate_version, Parser};
use config::Config;

use handlebars::no_escape;
use html_minifier::minify as html_minify;
use render::handlebars;
use rss_feed::Rss;
use std::fs::File;
use std::io::Write;
use std::net::SocketAddr;
use std::sync::Arc;
use tracing::{info, span};
use warp::{self, Filter};

#[derive(Parser)]
#[clap(version = crate_version!(), author = "Feng Yunlong <ylfeng@ir.hit.edu.cn>")]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Parser)]
enum SubCommand {
    #[clap(version = crate_version!(), author = "Feng Yunlong <ylfeng@ir.hit.edu.cn>", about = "Server serve.")]
    Serve(Serve),

    #[clap(version = crate_version!(),author = "Feng Yunlong <ylfeng@ir.hit.edu.cn>", about = "Build.")]
    Build(Build),

    #[clap(version = crate_version!(),author = "Feng Yunlong <ylfeng@ir.hit.edu.cn>", about = "Build PDF.")]
    Pdf(Pdf),
}

#[derive(Parser)]
struct Serve {
    #[clap(short, long, default_value = "127.0.0.1", about = "addr export")]
    addr: String,
    #[clap(short, long, default_value = "8080", about = "port export")]
    port: u16,
}

#[derive(Parser)]
struct Build {
    #[clap(short, long, about = "output filename")]
    output: Option<String>,
}

#[derive(Parser)]
struct Pdf {
    #[clap(short, long, about = "output filename")]
    output: Option<String>,
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
    let rss = Rss::feed_rss(&config).await?;
    let mut hbs = handlebars(&config)?;
    let statics_dir = config.statics_dir.as_str();

    match opts.subcmd {
        SubCommand::Serve(opt) => {
            let hbs_ref = Arc::new(hbs);

            let socks: SocketAddr = format!("{}:{}", opt.addr, opt.port).parse()?;

            let route = warp::get().and(warp::path::end()).map(move || {
                let result = hbs_ref
                    .render("index", &rss)
                    .unwrap_or_else(|e| e.to_string());
                warp::reply::html(result)
            });
            let static_files = warp::fs::dir(statics_dir.to_string());

            // GET / => rendered index templates
            // GET /... => statics_dir/...
            let routes = route.or(static_files);

            warp::serve(routes).run(socks).await;
        }
        SubCommand::Build(opt) => {
            info!("Copying static files!");
            copy_statics_to_target(&config)?;
            info!("Rendering templates!");
            let render_result = hbs.render("index", &rss)?;
            let render_result = if config.minify {
                info!("Minifying templates!");
                html_minify(render_result)?
            } else {
                render_result
            };
            let target_dir = std::path::Path::new(&config.target_dir);
            let default_path = &config
                .target_name
                .unwrap_or_else(|| "index.html".to_string());
            let index_path =
                target_dir.join(opt.output.unwrap_or_else(|| default_path.to_string()));
            let mut output_file = File::create(&index_path)?;
            output_file.write_all(render_result.as_bytes())?;
            println!("{} generated", index_path.to_string_lossy());
        }
        SubCommand::Pdf(opt) => {
            let target_dir = std::path::Path::new(&config.target_dir);
            let default_path = &config
                .target_name
                .unwrap_or_else(|| "output.tex".to_string());
            let index_path =
                target_dir.join(opt.output.unwrap_or_else(|| default_path.to_string()));
            let mut output_file = File::create(&index_path)?;
            info!("Rendering templates!");
            hbs.register_escape_fn(no_escape);
            let render_result = hbs.render("pdf", &rss)?;
            output_file.write_all(render_result.as_bytes())?;
            println!("{} generated", index_path.to_string_lossy());
        }
    }

    Ok(())
}
