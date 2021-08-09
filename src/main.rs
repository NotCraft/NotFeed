mod config;
mod render;
mod rhai_regex;
mod rss_feed;

#[macro_use]
extern crate actix_web;

use tracing::{info, span};
use tracing_subscriber;

use clap::{AppSettings, Clap};
use config::Config;

use fs_extra::dir::{copy, CopyOptions};
use render::handlebars;
use rss_feed::Rss;
use std::fs;
use std::fs::File;
use std::path::Path;

use serde_json::json;

use actix_web::dev::ServiceResponse;
use actix_web::middleware::errhandlers::{ErrorHandlerResponse, ErrorHandlers};
use actix_web::{body::Body, http::StatusCode, web, App, HttpResponse, HttpServer, Result};
use handlebars::Handlebars;

#[derive(Clap)]
#[clap(version = "1.0", author = "Feng Yunlong <ylfeng@ir.hit.edu.cn>")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    #[clap(subcommand)]
    subcmd: SubCommand,
}

#[derive(Clap)]
enum SubCommand {
    #[clap(
        version = "1.0",
        author = "Feng Yunlong <ylfeng@ir.hit.edu.cn>",
        about = "Server serve."
    )]
    Serve(Serve),

    #[clap(
        version = "1.0",
        author = "Feng Yunlong <ylfeng@ir.hit.edu.cn>",
        about = "Build."
    )]
    Build,
}

#[derive(Clap)]
struct Serve {
    #[clap(short, long, default_value = "127.0.0.1", about = "addr export")]
    addr: String,
    #[clap(short, long, default_value = "8080", about = "port export")]
    port: u16,
}

#[actix_web::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opts: Opts = Opts::parse();

    tracing_subscriber::fmt()
        .with_target(false)
        .try_init()
        .expect("Tracing init error!");
    let root = span!(tracing::Level::INFO, "<FEED>");
    let _enter = root.enter();

    let config = Config::new()?;

    info!("Copying Static Files!");
    fs::create_dir_all("target")?;
    if Path::new("static").exists() {
        let mut options = CopyOptions::new();
        options.content_only = true;
        copy(&config.statics_dir, "target", &options)?;
    }

    let rss = Rss::feed_rss(&config)?;
    let hbs = handlebars(&config)?;

    match opts.subcmd {
        SubCommand::Serve(serve) => {
            let handlebars_ref = web::Data::new(hbs);
            println!("Serve at http://{}:{}.", serve.addr, serve.port);
            HttpServer::new(move || {
                App::new()
                    .wrap(error_handlers())
                    .app_data(handlebars_ref.clone())
                    .data(rss.clone())
                    .service(index)
            })
            .bind(format!("{}:{}", serve.addr, serve.port))?
            .run()
            .await?;
        }
        SubCommand::Build => {
            let mut output_file = File::create("target/index.html")?;
            hbs.render_to_write("index", &rss, &mut output_file)?;
            println!("target/index.html generated");
        }
    }

    Ok(())
}

#[get("/")]
async fn index(hb: web::Data<Handlebars<'_>>, rss: web::Data<Rss>) -> HttpResponse {
    let body = hb.render("index", rss.get_ref()).unwrap();
    info!("Rendering Rss Page Done!");
    HttpResponse::Ok().body(body)
}

// Custom error handlers, to return HTML responses when an error occurs.
#[tracing::instrument]
fn error_handlers() -> ErrorHandlers<actix_web::body::Body> {
    ErrorHandlers::new().handler(StatusCode::NOT_FOUND, not_found)
}

// Error handler for a 404 Page not found error.
fn not_found<B>(res: ServiceResponse<B>) -> Result<ErrorHandlerResponse<B>> {
    let response = get_error_response(&res, "Page not found");
    Ok(ErrorHandlerResponse::Response(
        res.into_response(response.into_body()),
    ))
}

// Generic error handler.
fn get_error_response<B>(res: &ServiceResponse<B>, error: &str) -> HttpResponse<Body> {
    let request = res.request();

    // Provide a fallback to a simple plain text response in case an error occurs during the
    // rendering of the error page.
    let fallback = |e: &str| {
        HttpResponse::build(res.status())
            .content_type("text/plain")
            .body(e.to_string())
    };

    let hb = request
        .app_data::<web::Data<Handlebars>>()
        .map(|t| t.get_ref());
    match hb {
        Some(hb) => {
            let data = json!({
                "error": error,
                "status_code": res.status().as_str()
            });
            let body = hb.render("error", &data);

            match body {
                Ok(body) => HttpResponse::build(res.status())
                    .content_type("text/html")
                    .body(body),
                Err(_) => fallback(error),
            }
        }
        None => fallback(error),
    }
}
