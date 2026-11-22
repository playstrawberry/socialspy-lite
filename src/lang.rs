//! The script's texts: `lang/{en,fr,es,tr}.lang`, embedded and checked for the same keys while it compiles.
use sp_lang::{Catalog, Language};

static LANG: Catalog = sp_lang::catalog!("socialspy-lite");

/// The text of `key` in the language of `locale` (English for any other).
pub fn text(locale: &str, key: &str) -> String {
    let raw = LANG
        .get_or_en(Language::of_locale(locale), key)
        .unwrap_or_default();
    sp_lang::render(raw, &[], "")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_key_the_script_names_has_a_text_in_every_language() {
        let own = [
            "description",
            "permission_use",
            "command_socialspy",
            "players_only",
            "no_permission",
            "on",
            "off",
            "full",
        ];
        for key in own {
            for language in Language::ALL {
                assert!(
                    LANG.get(language, key).is_some_and(|t| !t.is_empty()),
                    "{key} in {}",
                    language.code()
                );
            }
        }
        assert_eq!(LANG.len(), own.len(), "a key nobody names");
        assert_eq!(text("xx_yy", "on"), text("en_us", "on"));
    }

    #[test]
    fn the_contracts_bound_and_node_are_the_scripts_own() {
        assert_eq!(crate::model::MAX_SPIES, sp_social::MAX_SPIES);
        assert_eq!(sp_social::SPY_NODE, "socialspy-lite:use");
        assert_eq!(sp_social::SPY_PLUGIN, "socialspy-lite");
    }
}
