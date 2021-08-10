mod config;
mod render;
mod rhai_regex;
mod rss_feed;

use clap::{crate_version, AppSettings, Clap};
use config::Config;
use fs_extra::copy_items;
use fs_extra::dir::{copy, get_dir_content, CopyOptions};
use html_minifier::{css::minify as css_minify, js::minify as js_minify, minify as html_minify};
use render::handlebars;
use rss_feed::Rss;
use std::fs;
use std::fs::File;
use std::io::Write;
use std::net::SocketAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tracing::{info, span};
use warp::{self, Filter};

#[derive(Clap)]
#[clap(version = crate_version!(), author = "Feng Yunlong <ylfeng@ir.hit.edu.cn>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    #[clap(version = crate_version!(), author = "Feng Yunlong <ylfeng@ir.hit.edu.cn>", about = "Server serve.")]
    Serve(Serve),

    #[clap(version = crate_version!(),author = "Feng Yunlong <ylfeng@ir.hit.edu.cn>", about = "Build.")]
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
    copy_statics_to_target(&config)?;

    let rss = Rss::feed_rss(&config).await?;
    let hbs = handlebars(&config)?;
    let statics_dir = config.statics_dir.as_str();

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
            let static_files = warp::fs::dir(statics_dir.to_string());

            // GET / => rendered index templates
            // GET /... => statics_dir/...
            let routes = route.or(static_files);

            warp::serve(routes).run(socks).await;
        }
        SubCommand::Build => {
            let mut output_file = File::create("target/index.html")?;
            info!("Rendering templates!");
            let render_result = hbs.render("index", &rss)?;
            let render_result = if config.minify {
                info!("Minifying templates!");
                html_minify(render_result)?
            } else {
                render_result
            };
            output_file.write_all(render_result.as_bytes())?;
            println!("target/index.html generated");
        }
    }

    Ok(())
}

fn copy_statics_to_target(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all("target")?;
    if Path::new(&config.statics_dir).exists() {
        let mut options = CopyOptions::new();
        options.content_only = !config.minify;
        options.overwrite = true;
        if config.minify {
            let from = get_dir_content(&config.statics_dir)?;
            let mut src_copy_items = vec![];
            for file in from.files {
                if file.ends_with(".js") {
                    let path = Path::new(&file);
                    if let Some(filename) = path.file_name() {
                        let file_content = fs::read_to_string(path)?;
                        let minify_content = js_minify(&file_content);
                        let output_path = PathBuf::from("target").join(filename);
                        let mut output_file = File::create(output_path)?;
                        output_file.write_all(minify_content.as_bytes())?;
                    }
                } else if file.ends_with(".css") {
                    let path = Path::new(&file);
                    if let Some(filename) = path.file_name() {
                        let file_content = fs::read_to_string(path)?;
                        let minify_content = css_minify(&file_content)?;
                        let output_path = PathBuf::from("target").join(filename);
                        let mut output_file = File::create(output_path)?;
                        output_file.write_all(minify_content.as_bytes())?;
                    }
                } else {
                    src_copy_items.push(file);
                }
            }
            copy_items(&src_copy_items, "target", &options)?;
        } else {
            copy(&config.statics_dir, "target", &options)?;
        }
    }
    Ok(())
}
