use aidoku::{
    alloc::{String, Vec, string::ToString as _},
    prelude::*,
};

/// Decode a base-N token string to its numeric index
fn decode_token(token: &str, base: usize) -> Option<usize> {
    let mut result: usize = 0;
    for ch in token.chars() {
        let digit = match ch {
            '0'..='9' => (ch as u8 - b'0') as usize,
            'a'..='z' => (ch as u8 - b'a' + 10) as usize,
            _ => return None,
        };
        if digit >= base {
            return None;
        }
        result = result.checked_mul(base)?.checked_add(digit)?;
    }
    Some(result)
}

/// Extract a single-quoted string starting right after the opening quote.
/// Returns (content, rest_after_closing_quote).
fn extract_single_quoted(s: &str) -> Option<(&str, &str)> {
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'\'' && (i == 0 || bytes[i - 1] != b'\\') {
            return Some((&s[..i], &s[i + 1..]));
        }
        i += 1;
    }
    None
}

/// Unpack a Dean Edwards packed JavaScript string.
///
/// Supports input like:
/// ```text
/// eval(function(p,a,c,k,e,d){...}('template',base,count,'kw1|kw2|...'.split('|'),0,{}))
/// ```
pub fn unpack(input: &str) -> Option<String> {
    // Locate the arguments: }('template', base, count, 'keywords'.split('|'), ...)
    let args_pos = input.find("}('")?;
    let after = &input[args_pos + 3..]; // skip }('

    // 1. template (single-quoted)
    let (template, rest) = extract_single_quoted(after)?;

    // 2. base
    let rest = rest.trim_start().strip_prefix(',')?.trim_start();
    let comma = rest.find(',')?;
    let base: usize = rest[..comma].trim().parse().ok()?;

    // 3. count (unused but must skip)
    let rest = &rest[comma + 1..];
    let comma = rest.find(',')?;

    // 4. keywords: 'kw1|kw2|...'.split('|')
    let rest = &rest[comma + 1..];
    let q = rest.find('\'')?;
    let (kw_str, _) = extract_single_quoted(&rest[q + 1..])?;
    let keywords: Vec<&str> = kw_str.split('|').collect();

    // Replace tokens in the template
    let chars: Vec<char> = template.chars().collect();
    let mut result = String::new();
    let mut i = 0;

    while i < chars.len() {
        if chars[i].is_ascii_alphanumeric() || chars[i] == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let token: String = chars[start..i].iter().collect();

            if let Some(idx) = decode_token(&token, base) {
                if idx < keywords.len() && !keywords[idx].is_empty() {
                    result.push_str(keywords[idx]);
                    continue;
                }
            }
            result.push_str(&token);
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }

    Some(result)
}

/// Extract image URLs from decoded `dm5imagefun` JavaScript.
///
/// The decoded JS looks like:
/// ```text
/// function dm5imagefun(){var cid=123;var key='abc';var pix="https://...";
/// var pvalue=["/1.jpg","/2.jpg"];for(...){pvalue[i]=pix+pvalue[i]+'?cid=123&key=abc'}
/// return pvalue}
/// ```
pub fn extract_image_urls(decoded: &str) -> Option<Vec<String>> {
    let pix = find_var_string(decoded, "pix")?;
    let cid = find_var_number(decoded, "cid")?;
    let key = find_var_string(decoded, "key")?;
    let pvalue = find_var_array(decoded, "pvalue")?;

    let urls = pvalue
        .iter()
        .map(|p| format!("{}{}?cid={}&key={}", pix, p, cid, key))
        .collect();

    Some(urls)
}

// ── helpers for extracting JS variables ──────────────────────────────

/// Find `var <name>="value"` or `var <name>='value'` and return `value`.
fn find_var_string(js: &str, name: &str) -> Option<String> {
    for sep in &["=\"", "='", "=\\'", "=\\\""] {
        let needle = format!("{}{}", name, sep);
        if let Some(pos) = js.find(&needle) {
            let after = &js[pos + needle.len()..];
            // The quote we are looking for is the last char of the separator,
            // but if it's an escaped quote like `\'` we want to find the next `\'`.
            // Let's just look for the first matching quote character (either ' or ").
            let quote_char = if sep.contains('\'') { '\'' } else { '"' };

            let mut end_pos = 0;
            let bytes = after.as_bytes();
            while end_pos < bytes.len() {
                if bytes[end_pos] == quote_char as u8 {
                    // check if it's escaped
                    if end_pos > 0 && bytes[end_pos - 1] == b'\\' {
                        // If we are matching an escaped quote pattern `=\'`, the closing quote will likely also be `\'`
                        if sep.contains('\\') {
                            // This is the closing escaped quote, strip the backslash from the value later
                            break;
                        } else {
                            // Just an escaped quote inside a normal string, ignore it
                            end_pos += 1;
                            continue;
                        }
                    }
                    break;
                }
                end_pos += 1;
            }

            if end_pos < bytes.len() {
                let mut value = String::from(&after[..end_pos]);
                // If it was an escaped quote string, the last character of the value might be a backslash right before the quote
                if sep.contains('\\') && value.ends_with('\\') {
                    value.pop();
                }
                return Some(value);
            }
        }
    }
    None
}

/// Find `var <name>=12345` and return the number as a string.
fn find_var_number(js: &str, name: &str) -> Option<String> {
    let needle = format!("{}=", name);
    let pos = js.find(&needle)?;
    let after = &js[pos + needle.len()..];
    let end = after
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after.len());
    if end == 0 {
        return None;
    }
    Some(String::from(&after[..end]))
}

/// Find `var <name>=["/a.jpg","/b.jpg"]` and return the entries.
fn find_var_array(js: &str, name: &str) -> Option<Vec<String>> {
    let needle = format!("{}=[", name);
    let pos = js.find(&needle)?;
    let after = &js[pos + needle.len()..];
    let end = after.find(']')?;
    let content = &after[..end];

    let mut entries = Vec::new();
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' || bytes[i] == b'\'' {
            let quote = bytes[i];
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != quote {
                i += 1;
            }
            entries.push(String::from(&content[start..i]));
            i += 1;
        } else {
            i += 1;
        }
    }

    Some(entries)
}

/// Extract a JS variable from the page HTML, e.g. `var DM5_CID=1217932;` or `var DM5_CID = 1217932;`
pub fn extract_dm5_var<'a>(html: &'a str, var_name: &str) -> Option<&'a str> {
    let needle = format!("var {}", var_name);
    let mut current_html = html;

    while let Some(pos) = current_html.find(&needle) {
        let after_var = &current_html[pos + needle.len()..];
        let next_char = after_var.chars().next().unwrap_or('\0');

        if next_char == '=' || next_char.is_whitespace() {
            if let Some(eq_pos) = after_var.find('=') {
                let after_eq = &after_var[eq_pos + 1..];
                if let Some(end) = after_eq.find(|c: char| c == ';' || c == '\r' || c == '\n') {
                    let value = after_eq[..end].trim().trim_matches('"').trim_matches('\'');
                    return Some(value);
                }
            }
        }
        current_html = &current_html[pos + needle.len()..];
    }
    None
}

/// Extract the `dm5_key` hidden input value from the page HTML.
pub fn extract_dm5_key(html: &str) -> Option<&str> {
    let needle = "id=\"dm5_key\" value=\"";
    let pos = html.find(needle)?;
    let after = &html[pos + needle.len()..];
    let end = after.find('"')?;
    Some(&after[..end])
}
