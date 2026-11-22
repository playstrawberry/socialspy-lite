//! The engine's translation registry through the plugin SDK (`i18n.load-translations`, `i18n.translate`). Only used
//! when [`crate::ENGINE_REGISTRY_READY`] is open, and by the red test.

use strawberry_plugin_api::common::Locale;
use strawberry_plugin_api::i18n;

use crate::{Language, Registry};

/// The registry of the engine this plugin runs in. Works only inside an engine (a `wasm32-wasip2` plugin).
#[derive(Clone, Copy, Debug, Default)]
pub struct EngineRegistry;

const fn locale(language: Language) -> Locale {
    match language {
        Language::En => Locale::EnUs,
        Language::Fr => Locale::FrFr,
        Language::Es => Locale::EsEs,
        Language::Tr => Locale::TrTr,
    }
}

impl Registry for EngineRegistry {
    fn load_translations(&mut self, namespace: &str, json: &str, language: Language) {
        i18n::load_translations(namespace, json, locale(language));
    }

    fn translate(&self, key: &str, language: Language) -> String {
        i18n::translate(key, locale(language))
    }
}
