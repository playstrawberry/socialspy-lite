//! `sp-lang`: a plugin's texts in en, fr, es and tr, and each player's language.
//!
//! - [`catalog!`] embeds `lang/{en,fr,es,tr}.lang` next to the plugin's `Cargo.toml` and checks them while the plugin
//!   compiles: a key missing from one file fails the build ([`parity`]). The files are indexed then too, so loading
//!   parses nothing.
//! - [`PlayerLanguages`] resolves a player's language once, at join and when the client's locale changes, or takes the
//!   one the server profile fixes (TR: everyone `tr`). A message is then one lookup.
//! - [`Texts`] answers a key in a language with `{0}`, `{1}`, ... and `{server_name}` (from the server profile) filled
//!   in. A key a language lacks reads English, never the raw key; parity makes that the rare case.
//! - The engine's translation registry (`load-translations`, `translate`) is where the texts belong, so that the plugin
//!   keeps no lookup table of its own. **It is not used yet**: the engine's registry does not keep languages apart yet, so
//!   loading `tr_tr` overwrites English. [`ENGINE_REGISTRY_READY`] is the gate; `tests/registry_tr_tr.rs` holds the red
//!   test that opens it. Until then [`Texts`] answers from the build-time index (static, nothing allocated at load),
//!   which goes in the change that opens the gate.

pub mod contract;
#[cfg(feature = "engine")]
pub mod engine;
pub mod parity;
mod players;
mod render;

pub use players::{LanguageMode, PlayerLanguages};
pub use render::render;

use parity::{Entry, LANGUAGES};

/// Whether `sp-lang` loads the texts into the engine's registry and reads them back from it.
///
/// `false` while the engine's registry does not keep languages apart yet. It turns `true` only in the
/// change that makes `tests/registry_tr_tr.rs` pass and removes its `#[ignore]`; a test fails if the gate is open
/// while that test is still marked red.
pub const ENGINE_REGISTRY_READY: bool = false;

/// The four languages every plugin ships.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, PartialOrd, Ord)]
pub enum Language {
    En = 0,
    Fr = 1,
    Es = 2,
    Tr = 3,
}

impl Language {
    pub const ALL: [Self; LANGUAGES] = [Self::En, Self::Fr, Self::Es, Self::Tr];

    /// `en`, `fr`, `es`, `tr`.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en",
            Self::Fr => "fr",
            Self::Es => "es",
            Self::Tr => "tr",
        }
    }

    /// The Minecraft locale the registry keeps this language under.
    #[must_use]
    pub const fn locale(self) -> &'static str {
        match self {
            Self::En => "en_us",
            Self::Fr => "fr_fr",
            Self::Es => "es_es",
            Self::Tr => "tr_tr",
        }
    }

    const fn index(self) -> usize {
        self as usize
    }

    /// The language of a client locale in any spelling the platform uses (`tr_tr`, `tr-TR`, `TrTr`, `Locale::TrTr`,
    /// `fr_ca`); any other language reads English.
    #[must_use]
    pub fn of_locale(locale: &str) -> Self {
        let name = locale.rsplit("::").next().unwrap_or(locale).trim();
        let mut letters = name.chars().take(2).map(|c| c.to_ascii_lowercase());
        match (letters.next(), letters.next()) {
            (Some('f'), Some('r')) => Self::Fr,
            (Some('e'), Some('s')) => Self::Es,
            (Some('t'), Some('r')) => Self::Tr,
            _ => Self::En,
        }
    }
}

/// A plugin's four lang files and their build-time indexes. Made by [`catalog!`].
#[derive(Debug)]
pub struct Catalog {
    namespace: &'static str,
    texts: [&'static str; LANGUAGES],
    index: [&'static [Entry]; LANGUAGES],
}

impl Catalog {
    #[doc(hidden)]
    #[must_use]
    pub const fn new(
        namespace: &'static str,
        texts: [&'static str; LANGUAGES],
        index: [&'static [Entry]; LANGUAGES],
    ) -> Self {
        Self {
            namespace,
            texts,
            index,
        }
    }

    /// The registry namespace: the plugin's name.
    #[must_use]
    pub const fn namespace(&self) -> &'static str {
        self.namespace
    }

    /// How many keys each file has (the same in all four).
    #[must_use]
    pub const fn len(&self) -> usize {
        self.index[0].len()
    }

    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// The raw text of `key` in `language` only, `None` when the key does not exist.
    #[must_use]
    pub fn get(&self, language: Language, key: &str) -> Option<&'static str> {
        let text = self.texts[language.index()];
        let index = self.index[language.index()];
        let bytes = key.as_bytes();
        let h = parity::hash(bytes, 0, bytes.len());
        let at = index
            .binary_search_by(|entry| parity::compare_key(entry, text.as_bytes(), bytes, h))
            .ok()?;
        let entry = index.get(at)?;
        text.get(entry.value_start as usize..entry.value_end as usize)
    }

    /// The raw text of `key` in `language`, else in English; `None` only for a key English lacks.
    #[must_use]
    pub fn get_or_en(&self, language: Language, key: &str) -> Option<&'static str> {
        self.get(language, key)
            .or_else(|| self.get(Language::En, key))
    }

    /// Every (key, text) of one language, in index order.
    pub fn entries(
        &self,
        language: Language,
    ) -> impl Iterator<Item = (&'static str, &'static str)> {
        let text = self.texts[language.index()];
        self.index[language.index()].iter().filter_map(move |e| {
            Some((
                text.get(e.key_start as usize..e.key_end as usize)?,
                text.get(e.value_start as usize..e.value_end as usize)?,
            ))
        })
    }

    /// One language as the JSON object `load-translations` takes (`{"key":"text",...}`).
    #[must_use]
    pub fn json(&self, language: Language) -> String {
        self.json_prefixed(language, "")
    }

    /// [`Catalog::json`] with every key behind `prefix`: a [`Library`]'s texts as they are loaded under the
    /// namespace of the script that uses the library.
    #[must_use]
    pub fn json_prefixed(&self, language: Language, prefix: &str) -> String {
        let mut out = String::with_capacity(
            self.texts[language.index()].len() + 16 + prefix.len() * self.len(),
        );
        out.push('{');
        for (i, (key, text)) in self.entries(language).enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('"');
            json_escaped(&mut out, prefix);
            json_escaped(&mut out, key);
            out.push('"');
            out.push(':');
            json_string(&mut out, text);
        }
        out.push('}');
        out
    }
}

fn json_string(out: &mut String, s: &str) {
    out.push('"');
    json_escaped(out, s);
    out.push('"');
}

fn json_escaped(out: &mut String, s: &str) {
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                const HEX: &[u8; 16] = b"0123456789abcdef";
                out.push_str("\\u00");
                out.push(char::from(HEX[(c as usize >> 4) & 0xF]));
                out.push(char::from(HEX[c as usize & 0xF]));
            }
            c => out.push(c),
        }
    }
}

/// The engine's translation registry, as `sp-lang` uses it. [`engine::EngineRegistry`] is the real one (feature
/// `engine`); tests bring their own.
pub trait Registry {
    /// `load-translations(namespace, json, locale)`.
    fn load_translations(&mut self, namespace: &str, json: &str, language: Language);
    /// `translate("namespace:key", locale)`.
    fn translate(&self, key: &str, language: Language) -> String;
}

/// A library's texts inside a script: the library's own catalog, loaded into the registry under the namespace of the
/// script that uses it (the script's [`Catalog::namespace`], the only namespace the engine lets a script write) with
/// every key behind `prefix`, and looked up the same way. So two scripts that use the same library never share an
/// entry, and the library's own name is never a registry namespace.
///
/// ```
/// static KIT_TEXTS: sp_lang::Catalog = sp_lang::catalog!(texts: "kit", "hi=Hi\n", "hi=Salut\n", "hi=Hola\n", "hi=Selam\n");
/// static KIT: sp_lang::Library = sp_lang::Library::new("kit.", &KIT_TEXTS);
/// static SCRIPT: sp_lang::Catalog = sp_lang::catalog!(texts: "parkour", "a=A\n", "a=A\n", "a=A\n", "a=A\n");
/// assert_eq!(KIT.registry_key(&SCRIPT, "hi"), "parkour:kit.hi");
/// ```
#[derive(Debug, Clone, Copy)]
pub struct Library {
    prefix: &'static str,
    catalog: &'static Catalog,
}

impl Library {
    /// `prefix` is the library's name and a dot (`creator-kit.`).
    #[must_use]
    pub const fn new(prefix: &'static str, catalog: &'static Catalog) -> Self {
        Self { prefix, catalog }
    }

    /// What every key of the library starts with in the script's namespace.
    #[must_use]
    pub const fn prefix(&self) -> &'static str {
        self.prefix
    }

    /// The library's own texts (read directly while the gate is closed, and as the fallback).
    #[must_use]
    pub const fn catalog(&self) -> &'static Catalog {
        self.catalog
    }

    /// The registry key of the library's `key` inside `script`: `<script namespace>:<prefix><key>`.
    #[must_use]
    pub fn registry_key(&self, script: &Catalog, key: &str) -> String {
        format!("{}:{}{key}", script.namespace, self.prefix)
    }
}

/// A plugin's texts, ready to answer: the catalog, the server profile's name, and the registry once the gate is open.
#[derive(Debug)]
pub struct Texts<R> {
    catalog: &'static Catalog,
    server_name: String,
    registry: Option<R>,
}

impl<R: Registry> Texts<R> {
    /// Loads the four files into the registry when [`ENGINE_REGISTRY_READY`], else keeps the registry untouched and
    /// answers from the catalog. Called once, in `on-load`.
    pub fn new(catalog: &'static Catalog, registry: R, server_name: impl Into<String>) -> Self {
        Self::with_gate(catalog, registry, server_name, ENGINE_REGISTRY_READY)
    }

    /// [`Texts::new`] with the gate given: for this crate's tests of the registry path. Plugins call `new`.
    #[doc(hidden)]
    pub fn with_gate(
        catalog: &'static Catalog,
        mut registry: R,
        server_name: impl Into<String>,
        gate_open: bool,
    ) -> Self {
        let registry = gate_open.then(|| {
            for language in Language::ALL {
                registry.load_translations(catalog.namespace, &catalog.json(language), language);
            }
            registry
        });
        Self {
            catalog,
            server_name: server_name.into(),
            registry,
        }
    }

    /// Adds a library's texts ([`Library`]): with the gate open they are loaded under this script's namespace, each
    /// key behind the library's prefix; read them with [`Texts::library_text`]. Called once, in `on-load`, after
    /// [`Texts::new`].
    #[must_use]
    pub fn with_library(mut self, library: &Library) -> Self {
        if let Some(registry) = &mut self.registry {
            for language in Language::ALL {
                registry.load_translations(
                    self.catalog.namespace,
                    &library.catalog.json_prefixed(language, library.prefix),
                    language,
                );
            }
        }
        self
    }

    /// Whether the texts come from the engine's registry.
    #[must_use]
    pub fn uses_registry(&self) -> bool {
        self.registry.is_some()
    }

    /// The server's name from the profile (`{server_name}`).
    #[must_use]
    pub fn server_name(&self) -> &str {
        &self.server_name
    }

    /// The text of `key` in `language`, with `{0}`, `{1}`, ... from `args` and `{server_name}` filled in. English
    /// when the language lacks the key; empty for a key English lacks too (a plugin bug the parity check cannot
    /// see, since the key is not in any file), never the raw key.
    #[must_use]
    pub fn text(&self, language: Language, key: &str, args: &[&str]) -> String {
        self.answer(
            || format!("{}:{key}", self.catalog.namespace),
            self.catalog,
            language,
            key,
            args,
        )
    }

    /// The text of a library's `key` ([`Texts::with_library`]): looked up under this script's namespace behind the
    /// library's prefix, else read from the library's own catalog, as [`Texts::text`] does.
    #[must_use]
    pub fn library_text(
        &self,
        library: &Library,
        language: Language,
        key: &str,
        args: &[&str],
    ) -> String {
        self.answer(
            || library.registry_key(self.catalog, key),
            library.catalog,
            language,
            key,
            args,
        )
    }

    /// The registry's text of the key `full` makes when the gate is open and it has one, else `catalog`'s text of
    /// `key` (the registry key is built only when the registry is used).
    fn answer(
        &self,
        full: impl FnOnce() -> String,
        catalog: &Catalog,
        language: Language,
        key: &str,
        args: &[&str],
    ) -> String {
        if let Some(registry) = &self.registry {
            let full = full();
            let answer = registry.translate(&full, language);
            // The registry answers the key itself when it has no text; read the catalog's English then.
            if !answer.is_empty() && !answer.eq_ignore_ascii_case(&full) {
                return render(&answer, args, &self.server_name);
            }
        }
        let raw = catalog.get_or_en(language, key).unwrap_or_default();
        render(raw, args, &self.server_name)
    }
}

/// Embeds the calling crate's `lang/{en,fr,es,tr}.lang` and fails the build when their keys differ.
///
/// ```ignore
/// static LANG: sp_lang::Catalog = sp_lang::catalog!("heads");
/// ```
///
/// `catalog!(texts: "ns", EN, FR, ES, TR)` takes the four texts directly (tests). A missing key:
///
/// ```compile_fail,E0080
/// static LANG: sp_lang::Catalog = sp_lang::catalog!(texts: "demo", "a=A\nb=B\n", "a=A\nb=B\n", "a=A\nb=B\n", "a=A\n");
/// ```
///
/// ```
/// static LANG: sp_lang::Catalog = sp_lang::catalog!(texts: "demo", "a=A\nb=B\n", "a=Af\nb=Bf\n", "b=Be\na=Ae\n", "a=At\nb=Bt\n");
/// assert_eq!(LANG.get(sp_lang::Language::Tr, "b"), Some("Bt"));
/// ```
#[macro_export]
macro_rules! catalog {
    ($namespace:expr) => {
        $crate::catalog!(
            texts: $namespace,
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/lang/en.lang")),
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/lang/fr.lang")),
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/lang/es.lang")),
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/lang/tr.lang"))
        )
    };
    (texts: $namespace:expr, $en:expr, $fr:expr, $es:expr, $tr:expr) => {{
        const TEXTS: [&str; $crate::parity::LANGUAGES] = [$en, $fr, $es, $tr];
        // A count reads every byte of its file, as an index does: thousands of keys pass the lint's step limit.
        #[allow(long_running_const_eval)]
        const N0: usize = $crate::parity::count(TEXTS[0]);
        #[allow(long_running_const_eval)]
        const N1: usize = $crate::parity::count(TEXTS[1]);
        #[allow(long_running_const_eval)]
        const N2: usize = $crate::parity::count(TEXTS[2]);
        #[allow(long_running_const_eval)]
        const N3: usize = $crate::parity::count(TEXTS[3]);
        #[allow(long_running_const_eval)]
        const I0: ([$crate::parity::Entry; N0], Option<$crate::parity::Problem>) = $crate::parity::index::<N0>(TEXTS[0]);
        #[allow(long_running_const_eval)]
        const I1: ([$crate::parity::Entry; N1], Option<$crate::parity::Problem>) = $crate::parity::index::<N1>(TEXTS[1]);
        #[allow(long_running_const_eval)]
        const I2: ([$crate::parity::Entry; N2], Option<$crate::parity::Problem>) = $crate::parity::index::<N2>(TEXTS[2]);
        #[allow(long_running_const_eval)]
        const I3: ([$crate::parity::Entry; N3], Option<$crate::parity::Problem>) = $crate::parity::index::<N3>(TEXTS[3]);
        const FAILURE: Option<$crate::parity::Failure> =
            $crate::parity::check(&TEXTS, [&I0.0, &I1.0, &I2.0, &I3.0], [I0.1, I1.1, I2.1, I3.1]);
        const _: () = if let Some(failure) = FAILURE {
            panic!("{}", failure.message())
        };
        const _: () = if let Some(failure) = FAILURE {
            panic!("{}", failure.subject(&TEXTS))
        };
        static E0: [$crate::parity::Entry; N0] = I0.0;
        static E1: [$crate::parity::Entry; N1] = I1.0;
        static E2: [$crate::parity::Entry; N2] = I2.0;
        static E3: [$crate::parity::Entry; N3] = I3.0;
        $crate::Catalog::new($namespace, TEXTS, [&E0, &E1, &E2, &E3])
    }};
}
