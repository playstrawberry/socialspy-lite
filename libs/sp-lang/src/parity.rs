//! The lang file format and its parity rule, written as `const fn`s so that [`catalog!`](crate::catalog)
//! checks a plugin's four files while the plugin compiles: a missing key fails the build.
//!
//! The format is the one the lang parity checker reads (and the plugins read): `key=text` lines, `#`
//! comments and blank lines skipped, spaces around a key and a text do not count, a leading UTF-8 byte
//! order mark is skipped. A key one language has and another lacks, a key written twice in one file and a
//! line with no key all fail. A crate on `sp-lang` is checked here only, so no crate has two parity checks.
//!
//! The work is O(n log n) per file: every key is hashed, the index is sorted by (hash, key) and the four
//! sorted indexes are walked side by side. Equal hashes are always compared byte for byte.

/// en, fr, es, tr: the order of every per-language array of this crate.
pub const LANGUAGES: usize = 4;

/// The files' names, in [`LANGUAGES`] order, as error messages print them.
pub const FILE_NAMES: [&str; LANGUAGES] = [
    "lang/en.lang",
    "lang/fr.lang",
    "lang/es.lang",
    "lang/tr.lang",
];

/// One `key=text` line of a file: byte ranges into that file's text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Entry {
    pub hash: u64,
    pub key_start: u32,
    pub key_end: u32,
    pub value_start: u32,
    pub value_end: u32,
    /// 1-based line number, for messages.
    pub line: u32,
}

const EMPTY: Entry = Entry {
    hash: 0,
    key_start: 0,
    key_end: 0,
    value_start: 0,
    value_end: 0,
    line: 0,
};

/// What is wrong with one file, or between two.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// The line has no `=` or nothing before it.
    NoKey { line: u32 },
    /// The key of this line was written before in the same file.
    Twice { line: u32 },
    /// The file lacks the key that `has` has on `line`.
    Lacks { has: usize, line: u32 },
    /// The file is larger than 4 GiB.
    TooLarge,
}

/// A parity failure: the file it is about and the problem.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Failure {
    /// Index into [`LANGUAGES`] order.
    pub language: usize,
    pub problem: Problem,
}

impl Failure {
    /// A fixed sentence for the build error. The key or line comes in a second error, [`Failure::subject`],
    /// because a `const` panic prints one `&str`.
    #[must_use]
    pub const fn message(&self) -> &'static str {
        match (self.problem, self.language) {
            (Problem::Lacks { .. }, 0) => {
                "sp-lang parity: lang/en.lang lacks a key another language file has (the key is in the next error)"
            }
            (Problem::Lacks { .. }, 1) => {
                "sp-lang parity: lang/fr.lang lacks a key another language file has (the key is in the next error)"
            }
            (Problem::Lacks { .. }, 2) => {
                "sp-lang parity: lang/es.lang lacks a key another language file has (the key is in the next error)"
            }
            (Problem::Lacks { .. }, _) => {
                "sp-lang parity: lang/tr.lang lacks a key another language file has (the key is in the next error)"
            }
            (Problem::Twice { .. }, 0) => {
                "sp-lang parity: lang/en.lang has a key twice (the key is in the next error)"
            }
            (Problem::Twice { .. }, 1) => {
                "sp-lang parity: lang/fr.lang has a key twice (the key is in the next error)"
            }
            (Problem::Twice { .. }, 2) => {
                "sp-lang parity: lang/es.lang has a key twice (the key is in the next error)"
            }
            (Problem::Twice { .. }, _) => {
                "sp-lang parity: lang/tr.lang has a key twice (the key is in the next error)"
            }
            (Problem::NoKey { .. }, 0) => {
                "sp-lang parity: lang/en.lang has a line with no key (the line is in the next error)"
            }
            (Problem::NoKey { .. }, 1) => {
                "sp-lang parity: lang/fr.lang has a line with no key (the line is in the next error)"
            }
            (Problem::NoKey { .. }, 2) => {
                "sp-lang parity: lang/es.lang has a line with no key (the line is in the next error)"
            }
            (Problem::NoKey { .. }, _) => {
                "sp-lang parity: lang/tr.lang has a line with no key (the line is in the next error)"
            }
            (Problem::TooLarge, _) => "sp-lang parity: a lang file is larger than 4 GiB",
        }
    }

    /// The key the failure is about (for `NoKey`, the whole line), taken from the file that has it.
    #[must_use]
    pub const fn subject<'a>(&self, texts: &[&'a str; LANGUAGES]) -> &'a str {
        let (file, line) = match self.problem {
            Problem::Lacks { has, line } => (has, line),
            Problem::Twice { line } | Problem::NoKey { line } => (self.language, line),
            Problem::TooLarge => return "",
        };
        if file >= LANGUAGES {
            return "";
        }
        let text = texts[file];
        let (start, end) = line_range(text.as_bytes(), line);
        let (start, end) = trim(text.as_bytes(), start, end);
        if let Some(eq) = find_eq(text.as_bytes(), start, end) {
            let (ks, ke) = trim(text.as_bytes(), start, eq);
            if ks < ke {
                return sub(text, ks, ke);
            }
        }
        sub(text, start, end)
    }
}

/// Lines a file declares (not blank, not a comment), valid or not: the size of its index.
#[must_use]
pub const fn count(text: &str) -> usize {
    let bytes = text.as_bytes();
    let mut i = bom(bytes);
    let mut n = 0;
    while i < bytes.len() {
        let end = line_end(bytes, i);
        let (s, e) = trim(bytes, i, end);
        if s < e && bytes[s] != b'#' {
            n += 1;
        }
        i = end + 1;
    }
    n
}

/// The file's index, sorted by (hash, key), with the first problem of the file if it has one. `N` is
/// [`count`] of the same text.
#[must_use]
pub const fn index<const N: usize>(text: &str) -> ([Entry; N], Option<Problem>) {
    let mut entries = [EMPTY; N];
    let problem = index_into(text, &mut entries);
    (entries, problem)
}

/// [`index`] into a slice of [`count`] entries (a slice of another length is filled or read only as far as both go).
pub const fn index_into(text: &str, entries: &mut [Entry]) -> Option<Problem> {
    let bytes = text.as_bytes();
    if bytes.len() > u32::MAX as usize {
        return Some(Problem::TooLarge);
    }
    let n_max = entries.len();
    let mut problem = None;
    let mut i = bom(bytes);
    let mut line = 0u32;
    let mut n = 0;
    while i < bytes.len() && n < n_max {
        line += 1;
        let end = line_end(bytes, i);
        let (s, e) = trim(bytes, i, end);
        if s < e && bytes[s] != b'#' {
            match find_eq(bytes, s, e) {
                Some(eq) => {
                    let (ks, ke) = trim(bytes, s, eq);
                    let (vs, ve) = trim(bytes, eq + 1, e);
                    if ks >= ke && problem.is_none() {
                        problem = Some(Problem::NoKey { line });
                    }
                    entries[n] = Entry {
                        hash: hash(bytes, ks, ke),
                        key_start: ks as u32,
                        key_end: ke as u32,
                        value_start: vs as u32,
                        value_end: ve as u32,
                        line,
                    };
                }
                None => {
                    if problem.is_none() {
                        problem = Some(Problem::NoKey { line });
                    }
                    entries[n] = Entry {
                        hash: 0,
                        key_start: s as u32,
                        key_end: s as u32,
                        value_start: s as u32,
                        value_end: s as u32,
                        line,
                    };
                }
            }
            n += 1;
        }
        i = end + 1;
    }
    sort(entries, bytes);
    if problem.is_none() {
        let mut k = 1;
        while k < n_max {
            if compare(&entries[k - 1], &entries[k], bytes, bytes) == 0 {
                let later = if entries[k].line > entries[k - 1].line {
                    entries[k].line
                } else {
                    entries[k - 1].line
                };
                problem = Some(Problem::Twice { line: later });
                break;
            }
            k += 1;
        }
    }
    problem
}

/// The parity rule over the four files and their indexes: the first failure, or `None`.
#[must_use]
pub const fn check(
    texts: &[&str; LANGUAGES],
    indexes: [&[Entry]; LANGUAGES],
    problems: [Option<Problem>; LANGUAGES],
) -> Option<Failure> {
    let mut l = 0;
    while l < LANGUAGES {
        if let Some(problem) = problems[l] {
            return Some(Failure {
                language: l,
                problem,
            });
        }
        l += 1;
    }
    // Every file against English, both ways: a key that only fr has shows as "en lacks it".
    let mut other = 1;
    while other < LANGUAGES {
        let a = indexes[0];
        let b = indexes[other];
        let ta = texts[0].as_bytes();
        let tb = texts[other].as_bytes();
        let (mut i, mut j) = (0, 0);
        while i < a.len() || j < b.len() {
            let order = if i == a.len() {
                1
            } else if j == b.len() {
                -1
            } else {
                compare(&a[i], &b[j], ta, tb)
            };
            if order == 0 {
                i += 1;
                j += 1;
            } else if order < 0 {
                return Some(Failure {
                    language: other,
                    problem: Problem::Lacks {
                        has: 0,
                        line: a[i].line,
                    },
                });
            } else {
                return Some(Failure {
                    language: 0,
                    problem: Problem::Lacks {
                        has: other,
                        line: b[j].line,
                    },
                });
            }
        }
        other += 1;
    }
    None
}

/// FNV-1a, 64 bits, over the key's bytes. Also used at run time to find a key.
#[must_use]
pub const fn hash(bytes: &[u8], start: usize, end: usize) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut i = start;
    while i < end {
        h ^= bytes[i] as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
        i += 1;
    }
    h
}

/// -1, 0 or 1: entries by hash, then by key bytes.
#[must_use]
pub const fn compare(a: &Entry, b: &Entry, ta: &[u8], tb: &[u8]) -> i32 {
    if a.hash != b.hash {
        return if a.hash < b.hash { -1 } else { 1 };
    }
    let (mut i, mut j) = (a.key_start as usize, b.key_start as usize);
    let (ie, je) = (a.key_end as usize, b.key_end as usize);
    while i < ie && j < je {
        if ta[i] != tb[j] {
            return if ta[i] < tb[j] { -1 } else { 1 };
        }
        i += 1;
        j += 1;
    }
    if i < ie {
        1
    } else if j < je {
        -1
    } else {
        0
    }
}

/// -1, 0 or 1: an entry against a key with its hash.
#[must_use]
pub fn compare_key(entry: &Entry, text: &[u8], key: &[u8], key_hash: u64) -> std::cmp::Ordering {
    entry.hash.cmp(&key_hash).then_with(|| {
        text.get(entry.key_start as usize..entry.key_end as usize)
            .unwrap_or_default()
            .cmp(key)
    })
}

const fn sort(entries: &mut [Entry], text: &[u8]) {
    // Heap sort: O(n log n) and no allocation, which a `const fn` needs.
    let n = entries.len();
    if n < 2 {
        return;
    }
    let mut start = n / 2;
    while start > 0 {
        start -= 1;
        sift(entries, start, n, text);
    }
    let mut end = n;
    while end > 1 {
        end -= 1;
        entries.swap(0, end);
        sift(entries, 0, end, text);
    }
}

const fn sift(entries: &mut [Entry], mut root: usize, end: usize, text: &[u8]) {
    loop {
        let mut child = 2 * root + 1;
        if child >= end {
            return;
        }
        if child + 1 < end && compare(&entries[child], &entries[child + 1], text, text) < 0 {
            child += 1;
        }
        if compare(&entries[root], &entries[child], text, text) >= 0 {
            return;
        }
        entries.swap(root, child);
        root = child;
    }
}

const fn bom(bytes: &[u8]) -> usize {
    if bytes.len() >= 3 && bytes[0] == 0xEF && bytes[1] == 0xBB && bytes[2] == 0xBF {
        3
    } else {
        0
    }
}

const fn line_end(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() && bytes[i] != b'\n' {
        i += 1;
    }
    i
}

/// The byte range of 1-based line `line` (counting every line, as `index` does).
const fn line_range(bytes: &[u8], line: u32) -> (usize, usize) {
    let mut i = bom(bytes);
    let mut at = 1;
    while i < bytes.len() {
        let end = line_end(bytes, i);
        if at == line {
            return (i, end);
        }
        at += 1;
        i = end + 1;
    }
    (bytes.len(), bytes.len())
}

const fn is_space(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\r' | b'\n' | 0x0B | 0x0C)
}

const fn trim(bytes: &[u8], mut start: usize, mut end: usize) -> (usize, usize) {
    while start < end && is_space(bytes[start]) {
        start += 1;
    }
    while end > start && is_space(bytes[end - 1]) {
        end -= 1;
    }
    (start, end)
}

const fn find_eq(bytes: &[u8], mut i: usize, end: usize) -> Option<usize> {
    while i < end {
        if bytes[i] == b'=' {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// `text[start..end]`, or "" when that is not a character boundary.
const fn sub(text: &str, start: usize, end: usize) -> &str {
    let bytes = text.as_bytes();
    if start > end || end > bytes.len() {
        return "";
    }
    let (_, tail) = bytes.split_at(start);
    let (middle, _) = tail.split_at(end - start);
    match core::str::from_utf8(middle) {
        Ok(s) => s,
        Err(_) => "",
    }
}
