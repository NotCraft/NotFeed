use crate::rhai_regex::{PlusPackage, RhaiMatch};
use crate::Config;
use handlebars::no_escape;
use handlebars::Handlebars;
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
    handlebars.register_templates_directory(".hbs", &config.templates_dir)?;

    for (name, script_path) in &config.scripts {
        info!("Loading Script: {} => {}", name, script_path);
        handlebars.register_script_helper_file(name, script_path)?;
    }

    info!("Building Handlebars Render Engine Done!");

    Ok(handlebars)
}
