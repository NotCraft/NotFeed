#![allow(non_snake_case)]

use rhai::plugin::*;
use rhai::{def_package, packages::StandardPackage};

def_package!(rhai:PlusPackage:"For Regex support.", lib, {
    StandardPackage::init(lib);
    combine_with_exported_module!(lib, "regex", regex_module);
});

#[derive(Debug, Clone)]
pub struct RhaiMatch {
    pub(crate) text: ImmutableString,
    pub(crate) start: usize,
    pub(crate) end: usize,
}

#[export_module]
mod regex_module {
    use crate::rhai_regex::RhaiMatch;
    use regex::Regex;
    use rhai::{Dynamic, EvalAltResult, ImmutableString, NativeCallContext, Position};

    #[rhai_fn(get = "text")]
    pub fn match_get_text(m: &mut RhaiMatch) -> ImmutableString {
        m.text.as_str().into()
    }

    #[rhai_fn(get = "start")]
    pub fn match_get_start(m: &mut RhaiMatch) -> usize {
        m.start
    }

    #[rhai_fn(get = "end")]
    pub fn match_get_end(m: &mut RhaiMatch) -> usize {
        m.end
    }

    #[rhai_fn(name = "Regex", return_raw)]
    pub fn regex_new(re: &str) -> Result<Regex, Box<EvalAltResult>> {
        Regex::new(re).map_err(|err| {
            EvalAltResult::ErrorInFunctionCall(
                "new_regex".to_string(),
                err.to_string(),
                "".into(),
                Position::NONE,
            )
            .into()
        })
    }

    #[rhai_fn(name = "find")]
    pub fn regex_find(regex: &mut Regex, text: &str) -> RhaiMatch {
        match regex.find(text) {
            None => RhaiMatch {
                text: Default::default(),
                start: 0,
                end: 0,
            },
            Some(res) => RhaiMatch {
                text: res.as_str().into(),
                start: res.start(),
                end: res.end(),
            },
        }
    }

    #[rhai_fn(name = "find")]
    pub fn regex_find_at(regex: &mut Regex, text: &str, start: usize) -> RhaiMatch {
        match regex.find_at(text, start) {
            None => RhaiMatch {
                text: Default::default(),
                start: 0,
                end: 0,
            },
            Some(res) => RhaiMatch {
                text: res.as_str().into(),
                start: res.start(),
                end: res.end(),
            },
        }
    }

    #[rhai_fn(name = "is_match")]
    pub fn regex_is_match(regex: &mut Regex, text: &str) -> bool {
        regex.is_match(text)
    }

    #[rhai_fn(name = "is_match")]
    pub fn regex_is_match_at(regex: &mut Regex, text: &str, start: usize) -> bool {
        regex.is_match_at(text, start)
    }

    #[rhai_fn(name = "replace")]
    pub fn regex_replace(regex: &mut Regex, text: &str, rep: &str) -> ImmutableString {
        regex.replace(text, rep).to_string().into()
    }

    #[rhai_fn(name = "replace")]
    pub fn regex_replacen(
        regex: &mut Regex,
        text: &str,
        limit: usize,
        rep: &str,
    ) -> ImmutableString {
        regex.replacen(text, limit, rep).to_string().into()
    }

    #[rhai_fn(name = "replace_all")]
    pub fn regex_replacen_all(regex: &mut Regex, text: &str, rep: &str) -> ImmutableString {
        regex.replace_all(text, rep).to_string().into()
    }

    #[rhai_fn(name = "split")]
    pub fn regex_splitn(regex: &mut Regex, text: &str, limit: usize) -> Dynamic {
        regex
            .splitn(text, limit)
            .map(|x| Dynamic::from(x.to_string()))
            .collect()
    }

    #[rhai_fn(name = "split")]
    pub fn regex_split(regex: &mut Regex, text: &str) -> Dynamic {
        regex
            .split(text)
            .map(|x| Dynamic::from(x.to_string()))
            .collect()
    }
}
