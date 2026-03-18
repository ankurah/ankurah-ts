/// Static exception table for name mapping beyond mechanical snake->camel conversion.
static STATIC_MAP: &[(&str, &str)] = &[
    ("fmt", "toString"),
    ("serialize", "encode"),
    ("deserialize", "decode"),
    ("eq", "equals"),
    ("ne", "notEquals"),
    ("partial_cmp", "compareTo"),
    ("clone", "clone"),
    ("default", "default"),
    ("drop", "drop"),
    ("from", "from"),
    ("try_from", "tryFrom"),
    ("into", "into"),
    ("new", "new"),
    ("next", "next"),
    ("deref", "deref"),
];

/// Convert a Rust snake_case name to the expected TS camelCase name.
pub fn rust_to_ts_name(rust_name: &str) -> String {
    for (rust, ts) in STATIC_MAP {
        if *rust == rust_name {
            return ts.to_string();
        }
    }
    snake_to_camel(rust_name)
}

/// Convert snake_case to camelCase.
fn snake_to_camel(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for (i, c) in s.chars().enumerate() {
        if c == '_' {
            if i == 0 || i == s.len() - 1 {
                result.push(c);
            } else {
                capitalize_next = true;
            }
        } else if capitalize_next {
            result.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(c);
        }
    }

    result
}

/// Types stay as PascalCase (no conversion needed).
pub fn rust_to_ts_type_name(rust_name: &str) -> String {
    rust_name.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snake_to_camel() {
        assert_eq!(snake_to_camel("my_function"), "myFunction");
        assert_eq!(snake_to_camel("get_value"), "getValue");
        assert_eq!(snake_to_camel("simple"), "simple");
        assert_eq!(snake_to_camel("a_b_c"), "aBC");
        assert_eq!(snake_to_camel("parse_selection"), "parseSelection");
        assert_eq!(snake_to_camel("_private"), "_private");
    }

    #[test]
    fn test_static_map() {
        assert_eq!(rust_to_ts_name("fmt"), "toString");
        assert_eq!(rust_to_ts_name("serialize"), "encode");
        assert_eq!(rust_to_ts_name("eq"), "equals");
        assert_eq!(rust_to_ts_name("try_from"), "tryFrom");
    }
}
