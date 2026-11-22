//! Each player's language, resolved once per player: at join and when the client's locale changes. A message then
//! costs one map read, or nothing at all when the server profile fixes the language.

use std::collections::HashMap;
use std::hash::Hash;

use crate::Language;

/// How the server picks a player's language (the server profile's language mode).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum LanguageMode {
    /// Everyone reads this language, whatever the client's locale (the TR server: `tr`).
    Fixed(Language),
    /// Each player reads the client's own locale; any language other than the four reads English.
    PerPlayer,
}

/// The language of every online player, keyed by the plugin's player id (a UUID). Bounded by the players online:
/// [`PlayerLanguages::quit`] drops the entry.
#[derive(Debug)]
pub struct PlayerLanguages<K> {
    mode: LanguageMode,
    players: HashMap<K, Language>,
}

impl<K: Eq + Hash> PlayerLanguages<K> {
    #[must_use]
    pub fn new(mode: LanguageMode) -> Self {
        Self {
            mode,
            players: HashMap::new(),
        }
    }

    #[must_use]
    pub fn mode(&self) -> LanguageMode {
        self.mode
    }

    /// A player joined with this client locale.
    pub fn join(&mut self, player: K, client_locale: &str) {
        if self.mode == LanguageMode::PerPlayer {
            self.players
                .insert(player, Language::of_locale(client_locale));
        }
    }

    /// The client changed its locale.
    pub fn locale_changed(&mut self, player: K, client_locale: &str) {
        self.join(player, client_locale);
    }

    /// The player left.
    pub fn quit(&mut self, player: &K) {
        self.players.remove(player);
    }

    /// The player's language: the fixed one, the one resolved at join, or English for a player not seen (console).
    #[must_use]
    pub fn get(&self, player: &K) -> Language {
        match self.mode {
            LanguageMode::Fixed(language) => language,
            LanguageMode::PerPlayer => self.players.get(player).copied().unwrap_or(Language::En),
        }
    }

    /// Players held (0 in a fixed mode).
    #[must_use]
    pub fn len(&self) -> usize {
        self.players.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.players.is_empty()
    }
}
