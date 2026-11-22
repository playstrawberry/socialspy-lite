//! What a translation registry must do before `sp-lang` loads into it: keep each language's texts apart. The engine's
//! registry does not yet; `tests/registry_tr_tr.rs` runs this check on it and is kept red until it does.

use crate::{Language, Registry};

/// The namespace the check writes under; no plugin uses it.
pub const NAMESPACE: &str = "sp_lang_contract";

const TEXTS: [(Language, &str); 4] = [
    (Language::En, "Hello"),
    (Language::Fr, "Bonjour"),
    (Language::Es, "Hola"),
    (Language::Tr, "Merhaba"),
];

/// Loads one key in en_us, fr_fr, es_es and tr_tr (English first, as `sp-lang` does), then reads each back. `Err`
/// names the first language that reads another language's text.
///
/// # Errors
/// The first locale whose text is not its own.
pub fn check<R: Registry>(registry: &mut R) -> Result<(), String> {
    for (language, text) in TEXTS {
        registry.load_translations(NAMESPACE, &format!("{{\"greeting\":\"{text}\"}}"), language);
    }
    let key = format!("{NAMESPACE}:greeting");
    for (language, text) in TEXTS {
        let answer = registry.translate(&key, language);
        if answer != text {
            return Err(format!(
                "{} reads {answer:?}, expected {text:?}",
                language.locale()
            ));
        }
    }
    Ok(())
}
