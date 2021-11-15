use crate::rhai_ext::{PlusPackage, RhaiMatch};
use crate::utils::{command_escape, remove_unpair, PDF_SRC, TEMPLATES_SRC};
use crate::Config;
use chrono::{DateTime, SecondsFormat, Utc};
use handlebars::Handlebars;
use handlebars::{no_escape, Context, Helper, Output, RenderContext, RenderError};
use html_escape::decode_html_entities;
use latex2mathml::replace;
use regex::Regex;
use rhai::packages::Package;
use tracing::info;

#[cfg(feature = "handlebars_misc_helpers")]
use handlebars_misc_helpers::setup_handlebars;

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
    handlebars.register_helper("build_time", Box::new(build_time_helper));
    handlebars.register_helper("time_format", Box::new(time_format_helper));
    handlebars.register_helper("latex_render", Box::new(latex_render_helper));
    handlebars.register_helper("latex_escape", Box::new(latex_escape_helper));
    #[cfg(feature = "handlebars_misc_helpers")]
    setup_handlebars(&mut handlebars);

    handlebars.register_escape_fn(no_escape);
    handlebars.register_template_string("pdf", PDF_SRC)?;
    handlebars.register_template_string("index", TEMPLATES_SRC)?;
    handlebars.register_templates_directory(".hbs", &config.templates_dir)?;

    for (name, script_path) in &config.scripts {
        info!("Loading Script: {} => {}", name, script_path);
        handlebars.register_script_helper_file(name, script_path)?;
    }

    info!("Building Handlebars Render Engine Done!");

    Ok(handlebars)
}

fn time_format_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    // get parameter from helper or throw an error
    let datetime: DateTime<Utc> = h
        .param(0)
        .and_then(|v| serde_json::from_value(v.value().clone()).unwrap())
        .ok_or_else(|| {
            RenderError::new("Param 0 (datetime) is required for time format helper.")
        })?;

    let fmt = h.param(1).and_then(|v| v.value().as_str());
    let rendered = match fmt {
        None => datetime.to_string(),
        Some("rfc2822") => datetime.to_rfc2822(),
        Some("rfc3339") => datetime.to_rfc3339(),
        Some("rfc3339_opts") => {
            let secform = h
                .hash_get("secform")
                .and_then(|v| v.value().as_str())
                .ok_or_else(|| {
                    RenderError::new(
                        "Param secform in [Secs,Millis,Micros,Nanos,AutoSi] required for datetime helper.",
                    )
                })?;

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
                .ok_or_else(|| RenderError::new("Param use_z in required for datetime helper."))?;

            datetime.to_rfc3339_opts(secform, use_z)
        }
        Some(fmt) => datetime.format(fmt).to_string(),
    };
    out.write(&rendered)?;
    Ok(())
}

fn latex_escape_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let text = h
        .param(0)
        .and_then(|v| v.value().as_str())
        .ok_or_else(|| RenderError::new("Param 0 is required for latex render helper."))?;
    let decoded_html = decode_html_entities(text);
    let decoded_html = decoded_html.replace("<p>", "").replace("</p>", "");
    let decoded_html = remove_unpair(&decoded_html, '{', '}');
    let text = command_escape(&decoded_html);
    let text = v_latexescape::escape(&text).to_string();
    out.write(&text)?;
    Ok(())
}

fn latex_render_helper(
    h: &Helper,
    _: &Handlebars,
    _: &Context,
    _: &mut RenderContext,
    out: &mut dyn Output,
) -> Result<(), RenderError> {
    let text = h
        .param(0)
        .and_then(|v| v.value().as_str())
        .ok_or_else(|| RenderError::new("Param 0 is required for latex render helper."))?;
    let text = command_escape(text);
    let text = if let Ok(x) = replace(&text) {
        if x.contains("[PARSE ERROR:") {
            text
        } else {
            x
        }
    } else {
        text
    };
    out.write(&text)?;
    Ok(())
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
                .ok_or_else(|| {
                    RenderError::new(
                "Param secform in [Secs,Millis,Micros,Nanos,AutoSi] required for datetime helper.",
            )
                })?;

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
                .ok_or_else(|| RenderError::new("Param use_z in required for datetime helper."))?;

            datetime.to_rfc3339_opts(secform, use_z)
        }
        Some(fmt) => datetime.format(fmt).to_string(),
    };
    out.write(&rendered)?;
    Ok(())
}
