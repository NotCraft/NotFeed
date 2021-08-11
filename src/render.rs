use crate::rhai_regex::{PlusPackage, RhaiMatch};
use crate::Config;
use chrono::{SecondsFormat, Utc};
use handlebars::Handlebars;
use handlebars::{no_escape, Context, Helper, Output, RenderContext, RenderError};
use pcre2::bytes::{Match, RegexBuilder};
use regex::Regex;
use rhai::packages::Package;
use tracing::info;

pub fn handlebars(config: &Config) -> Result<Handlebars<'static>, Box<dyn std::error::Error>> {
    info!("Building Script Engine!");
    let mut engine = rhai::Engine::new();
    engine
        .register_type_with_name::<Regex>("Regex")
        .register_type_with_name::<RhaiMatch>("Match");

    info!("Building Script Engine Done!");

    engine.register_global_module(PlusPackage::new().as_shared_module());

    info!("Building Handlebars Render Engine!");
    let mut handlebars = Handlebars::new();

    handlebars.set_engine(engine);
    handlebars.set_dev_mode(true);
    handlebars.register_escape_fn(no_escape);
    handlebars.register_helper("build_time", Box::new(build_time_helper));
    handlebars.register_helper("katex_render", Box::new(katex_render_helper));
    handlebars.register_templates_directory(".hbs", &config.templates_dir)?;

    for (name, script_path) in &config.scripts {
        info!("Loading Script: {} => {}", name, script_path);
        handlebars.register_script_helper_file(name, script_path)?;
    }

    info!("Building Handlebars Render Engine Done!");

    Ok(handlebars)
}

const LATEX_REGEX_STR: &str = r"(?<!\\)(?:((?<!\$)\${1,2}(?!\$))|(\\\()|(\\\[)|(\\begin\{equation\}))(?(1)(.*?)(?<!\\)(?<!\$)\1(?!\$)|(?:(.*(?R)?.*)(?<!\\)(?:(?(2)\\\)|(?(3)\\\]|\\end\{equation\})))))";

use lazy_static::lazy_static;
use std::borrow::Cow;
use std::option::Option::Some;

lazy_static! {
    static ref LATEX_REGEX: pcre2::bytes::Regex = {
        RegexBuilder::new()
            .jit(true)
            .build(LATEX_REGEX_STR)
            .unwrap()
    };
}

fn katex_render_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let text = h
        .param(0)
        .and_then(|v| v.value().as_str())
        .ok_or(RenderError::new(
            "Param 0 is required for katex render helper.",
        ))?;
    let text = tex_replace(text);
    out.write(&text)?;
    Ok(())
}

fn tex_replace(text: &str) -> Cow<str> {
    // The slower path, which we use if the replacement needs access to
    // capture groups.
    let mut it = LATEX_REGEX.captures_iter(text.as_bytes()).peekable();
    if it.peek().is_none() {
        return Cow::Borrowed(text);
    }
    let mut new = String::with_capacity(text.len());
    let mut last_match = 0;
    for cap in it {
        // unwrap on 0 is OK because captures only reports matches
        let cap = cap.unwrap();
        let m = cap.get(0).unwrap();
        new.push_str(&text[last_match..m.start()]);

        let rendered = if let Some(x) = cap.get(5) {
            let text = std::str::from_utf8(x.as_bytes()).unwrap();
            katex::render(text)
        } else if let Some(x) = cap.get(6) {
            let text = std::str::from_utf8(x.as_bytes()).unwrap();
            let opts = katex::Opts::builder().display_mode(true).build().unwrap();
            katex::render_with_opts(text, opts)
        } else {
            Ok(String::new())
        };

        if let Ok(txt) = rendered {
            new.push_str(&txt)
        };

        last_match = m.end();
    }
    new.push_str(&text[last_match..]);
    Cow::Owned(new)
}

fn build_time_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    // get parameter from helper or throw an error
    let fmt = h.param(0).and_then(|v| v.value().as_str());
    let datetime = Utc::now();
    let rendered = match fmt {
        None => datetime.to_string(),
        Some("rfc2822") => datetime.to_rfc2822(),
        Some("rfc3339") => datetime.to_rfc3339(),
        Some("rfc3339_opts") => {
            let secform = h
                .hash_get("secform")
                .and_then(|v| v.value().as_str())
                .ok_or(RenderError::new(
                "Param secform in [Secs,Millis,Micros,Nanos,AutoSi] required for datetime helper.",
            ))?;

            let secform = match secform {
                "Secs" => SecondsFormat::Secs,
                "Millis" => SecondsFormat::Millis,
                "Micros" => SecondsFormat::Micros,
                "Nanos" => SecondsFormat::Nanos,
                "AutoSi" => SecondsFormat::AutoSi,
                _ => SecondsFormat::Secs,
            };

            let use_z = h
                .hash_get("use_z")
                .and_then(|v| v.value().as_bool())
                .ok_or(RenderError::new(
                    "Param use_z in required for datetime helper.",
                ))?;

            datetime.to_rfc3339_opts(secform, use_z)
        }
        Some(fmt) => datetime.format(fmt).to_string(),
    };
    out.write(&rendered)?;
    Ok(())
}
