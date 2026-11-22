//! Filling in a text: `{0}`, `{1}`, ... from the arguments and `{server_name}` from the server profile.

/// `template` with `{0}`, `{1}`, ... replaced by `args` and `{server_name}` by `server_name`, in one pass and one
/// allocation. A placeholder with no argument, and any other brace, stays as written.
#[must_use]
pub fn render(template: &str, args: &[&str], server_name: &str) -> String {
    let mut out =
        String::with_capacity(template.len() + args.iter().map(|a| a.len()).sum::<usize>());
    let mut rest = template;
    while let Some(open) = rest.find('{') {
        out.push_str(&rest[..open]);
        let after = &rest[open + 1..];
        let Some(close) = after.find('}') else {
            out.push_str(&rest[open..]);
            return out;
        };
        let name = &after[..close];
        let value = if name == "server_name" {
            Some(server_name)
        } else if !name.is_empty() && name.bytes().all(|b| b.is_ascii_digit()) {
            name.parse::<usize>()
                .ok()
                .and_then(|i| args.get(i).copied())
        } else {
            None
        };
        match value {
            Some(value) => out.push_str(value),
            None => out.push_str(&rest[open..open + close + 2]),
        }
        rest = &after[close + 1..];
    }
    out.push_str(rest);
    out
}
