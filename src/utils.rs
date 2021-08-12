use crate::config::Config;
use fs_extra::copy_items;
use fs_extra::dir::{copy, get_dir_content, CopyOptions};
use html_escape::decode_html_entities;
use html_minifier::{css::minify as css_minify, js::minify as js_minify};
use std::fs;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};

#[macro_export]
macro_rules! crate_name {
    () => {
        env!("CARGO_PKG_NAME")
    };
}

#[macro_export]
macro_rules! crate_homepage {
    () => {
        env!("CARGO_PKG_HOMEPAGE")
    };
}

pub(crate) const TEMPLATES_SRC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/system-templates/index.hbs"
));

pub(crate) const PDF_SRC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/system-templates/pdf.tex"
));

pub(crate) const STATIC_CSS_SRC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/system-statics/index.css"
));

pub(crate) const STATIC_JS_SRC: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/system-statics/index.js"
));

pub(crate) const STATIC_ICO_SRC: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/vendor/system-statics/favicon.ico"
));

pub fn remove_unpair(s: &str, opening: char, closing: char) -> String {
    let mut removed = vec![];
    let chars: Vec<char> = s.chars().collect();
    remove_unpair_inner(chars, &mut removed, opening, closing, opening, 0, 0);
    removed.iter().collect()
}

fn remove_unpair_inner(
    s: Vec<char>,
    removed: &mut Vec<char>,
    opening: char,
    closing: char,
    top_opening: char,
    mut i: usize,
    last_removed: usize,
) {
    let mut count = 0;

    while i < s.len() && count >= 0 {
        if s[i] == opening {
            count += 1;
        } else if s[i] == closing {
            count -= 1
        }
        i += 1;
    }

    if count >= 0 {
        let mut s = s;
        s.reverse();
        if opening == top_opening {
            remove_unpair_inner(s, removed, closing, opening, top_opening, 0, 0);
        } else {
            removed.append(&mut s);
        }
    } else {
        for j in last_removed..i {
            if s[j] == closing && (j == 0 || s[j - 1] != closing) {
                let mut new_s = s[0..j].to_vec();
                let mut new_s2 = s[j + 1..].to_vec();
                new_s.append(&mut new_s2);
                remove_unpair_inner(new_s, removed, opening, closing, top_opening, i - 1, j);
            }
        }
    }
}

use lazy_static::lazy_static;
lazy_static! {
    static ref LATEX_REGEX: regex::Regex =
        regex::Regex::new(r"(?P<pre>[^\\])(?P<tex>[%&_])").unwrap();
}

pub fn latex_escape(input: &str) -> String {
    let decoded_html = decode_html_entities(input);
    let decoded_html = decoded_html
        .trim_start_matches("<p>")
        .trim_end_matches("</p>");
    let decoded_html = remove_unpair(decoded_html, '{', '}');
    LATEX_REGEX
        .replace_all(&decoded_html, "$pre\\$tex")
        .to_string()
}

pub fn copy_statics_to_target(config: &Config) -> Result<(), Box<dyn std::error::Error>> {
    fs::create_dir_all(&config.target_dir)?;
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
                        let output_path = PathBuf::from(&config.target_dir).join(filename);
                        let mut output_file = File::create(output_path)?;
                        output_file.write_all(minify_content.as_bytes())?;
                    }
                } else if file.ends_with(".css") {
                    let path = Path::new(&file);
                    if let Some(filename) = path.file_name() {
                        let file_content = fs::read_to_string(path)?;
                        let minify_content = css_minify(&file_content)?;
                        let output_path = PathBuf::from(&config.target_dir).join(filename);
                        let mut output_file = File::create(output_path)?;
                        output_file.write_all(minify_content.as_bytes())?;
                    }
                } else {
                    src_copy_items.push(file);
                }
            }
            copy_items(&src_copy_items, &config.target_dir, &options)?;
        } else {
            copy(&config.statics_dir, &config.target_dir, &options)?;
        }
    }
    let css_path = Path::new(&config.target_dir).join("index.css");
    if !css_path.exists() {
        let minify_content = css_minify(STATIC_CSS_SRC)?;
        let mut output_file = File::create(&css_path)?;
        output_file.write_all(minify_content.as_bytes())?;
    }
    let js_path = Path::new(&config.target_dir).join("index.js");
    if !js_path.exists() {
        let minify_content = js_minify(STATIC_JS_SRC);
        let mut output_file = File::create(&js_path)?;
        output_file.write_all(minify_content.as_bytes())?;
    }
    let ico_path = Path::new(&config.target_dir).join("favicon.ico");
    if !ico_path.exists() {
        let mut output_file = File::create(&ico_path)?;
        output_file.write_all(STATIC_ICO_SRC)?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_regex() {
        assert_eq!(
            LATEX_REGEX.replace_all("98.12% which is 1.62%", "$pre\\$tex"),
            "98.12\\% which is 1.62\\%"
        );
        assert_eq!(
            LATEX_REGEX.replace_all("98.12\\% which is 1.62%", "$pre\\$tex"),
            "98.12\\% which is 1.62\\%"
        );
    }

    #[test]
    fn test_remove() {
        assert_eq!(remove_unpair("(((()xx)", '(', ')'), "(()xx)");
        assert_eq!(remove_unpair("(((()xx))", '(', ')'), "((()xx))");
        assert_eq!(remove_unpair("(())", '(', ')'), "(())");
    }
}
