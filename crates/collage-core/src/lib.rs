//! Shared functions for [`collage`] and [`collage-macros`].
//!
//! [`collage`]: https://crates.io/crates/collage
//! [`collage-macros`]: https://crates.io/crates/collage-macros

#![no_std]
#![forbid(unsafe_code)]

pub fn is_valid_attr_key(attr: &str) -> bool {
    let attr = attr.split_once('=').map(|(k, _)| k).unwrap_or_else(|| attr);
    let mut chars = attr.chars();
    chars
        .next()
        .is_some_and(|c| c.is_alphanumeric() || c == '_' || c == ':')
        && chars.all(|c| c.is_alphanumeric() || c == '_' || c == ':' || c == '-' || c == '.')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_valid() {
        assert!(is_valid_attr_key("id"));
        assert!(is_valid_attr_key("data-var"));
        assert!(!is_valid_attr_key("'data-var"));
        assert!(is_valid_attr_key(r#"id="foo""#));
    }
}
