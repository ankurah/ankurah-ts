//! What a hand-written TypeScript class DECLARES, as against what its text
//! merely mentions.
//!
//! For: a `[provided_impls]` entry claims a file declares a member the emitter
//! will call — `debug()`, `toJSON()`, `static fromJson(..)` — and the check that
//! keeps that claim honest reads the file. Reading it as TEXT is not enough. A
//! body containing `contains("debug()")` over the whole class is satisfied by
//! any `x.debug()` a method writes, so deleting the declaration and leaving one
//! call behind kept the claim green; and it is satisfied by `static debug()`,
//! which the emission — which writes `instance.debug()` — never reaches. So the
//! claim is checked against a DECLARATION: a member written at the class body's
//! own top level, carrying the kind (static or instance) and the parameter list
//! it was written with.
//!
//! What this does NOT model, each a claim that FAILS rather than passing
//! wrongly: a method whose declaration is a field holding an arrow function
//! (`debug = () => ".."` is callable at run time and is reported here as a
//! field); a member added to the prototype from outside the class; a decorator
//! before a member. A file that needs one of those has to say so, and the gate
//! saying "no such declaration" is the safe direction to be wrong in.

use super::code_only;

/// One member written at a class body's top level.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Member {
    pub name: String,
    pub is_static: bool,
    /// The text between a method's parentheses. `None` where no parameter list
    /// was written at all, which is a field rather than a method.
    pub params: Option<String>,
}

impl Member {
    /// A method callable as `instance.<name>(..)`.
    pub fn is_instance_method(&self, name: &str) -> bool {
        !self.is_static && self.name == name && self.params.is_some()
    }

    /// A method callable as `Class.<name>(..)`.
    pub fn is_static_method(&self, name: &str) -> bool {
        self.is_static && self.name == name && self.params.is_some()
    }

    /// A method written with an empty parameter list.
    pub fn takes_nothing(&self) -> bool {
        self.params.as_deref().is_some_and(|p| p.trim().is_empty())
    }
}

/// Every member `export class <class>` declares at its own top level, or `None`
/// where the text declares no class of that name.
pub fn class_members(text: &str, class: &str) -> Option<Vec<Member>> {
    Some(top_level_members(&class_body(text, class)?))
}

/// Does the class declare an instance method of this name taking nothing?
pub fn declares_nullary_method(members: &[Member], name: &str) -> bool {
    members.iter().any(|m| m.is_instance_method(name) && m.takes_nothing())
}

/// Does the class declare an instance method of this name, whatever it takes?
pub fn declares_instance_method(members: &[Member], name: &str) -> bool {
    members.iter().any(|m| m.is_instance_method(name))
}

/// Does the class declare a static method of this name, whatever it takes?
pub fn declares_static_method(members: &[Member], name: &str) -> bool {
    members.iter().any(|m| m.is_static_method(name))
}

/// The body of one `export class` in a hand-written file, or `None` where the
/// file declares no class of that name.
///
/// Brace depth from the class's own `{`, so a nested class or an object literal
/// inside a method does not end it early. The file is read as CODE first
/// (`common::code_only`), so a member named only in a comment or inside a string
/// does not satisfy a check — which is the whole point of reading the file
/// rather than trusting the entry — and a brace inside a string does not end the
/// class early.
pub fn class_body(text: &str, class: &str) -> Option<String> {
    let text = code_only(text);
    let head = format!("export class {}", class);
    // The class name has to END there: `export class Entity` must not match
    // `export class EntityId`.
    let mut from = 0usize;
    let start = loop {
        let at = text[from..].find(&head)? + from;
        let after = text[at + head.len()..].chars().next()?;
        if !(after.is_alphanumeric() || after == '_') {
            break at;
        }
        from = at + head.len();
    };
    let open = text[start..].find('{')? + start;
    let mut depth = 0usize;
    for (at, ch) in text[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(text[open + 1..open + at].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// Cut a class body into one chunk per member and read each one.
///
/// A member ends at the `;` that terminates it or at the `}` that closes the
/// block it wrote — its method body, or an object literal it initialises a field
/// with — whichever comes first at the body's own nesting level. Anything deeper
/// than that level is inside a member and is not a declaration of this class.
fn top_level_members(body: &str) -> Vec<Member> {
    let mut out = Vec::new();
    let mut chunk = String::new();
    let (mut braces, mut parens, mut bracks) = (0i32, 0i32, 0i32);
    let flat = |b: i32, p: i32, k: i32| b <= 0 && p <= 0 && k <= 0;
    for ch in body.chars() {
        match ch {
            '{' => braces += 1,
            '}' => braces -= 1,
            '(' => parens += 1,
            ')' => parens -= 1,
            '[' => bracks += 1,
            ']' => bracks -= 1,
            _ => {}
        }
        if ch == ';' && flat(braces, parens, bracks) {
            out.extend(read_member(&chunk));
            chunk.clear();
            continue;
        }
        chunk.push(ch);
        if ch == '}' && flat(braces, parens, bracks) {
            out.extend(read_member(&chunk));
            chunk.clear();
        }
    }
    out.extend(read_member(&chunk));
    out
}

/// The words that can stand in front of a member's name. Each is a modifier
/// only where a NAME follows it: `get(key)` declares a member called `get`.
const MODIFIERS: [&str; 11] =
    ["static", "public", "private", "protected", "readonly", "async", "abstract", "override", "declare", "get", "set"];

/// Read one member out of its chunk: the modifiers, the name, and — where one
/// was written — the parameter list.
fn read_member(chunk: &str) -> Option<Member> {
    let mut rest = chunk.trim_start();
    let mut is_static = false;
    let name = loop {
        rest = rest.trim_start();
        // A generator's `*`, and a private name's `#`, sit against the name.
        rest = rest.strip_prefix('*').unwrap_or(rest).trim_start();
        let private = rest.starts_with('#');
        let word_from = if private { &rest[1..] } else { rest };
        let word: String = word_from.chars().take_while(is_name_char).collect();
        if word.is_empty() {
            return None;
        }
        let after = word_from[word.len()..].trim_start();
        let names_something =
            after.chars().next().is_some_and(|c| is_name_char(&c) || c == '#' || c == '*' || c == '\'' || c == '"');
        if !private && MODIFIERS.contains(&word.as_str()) && names_something {
            is_static |= word == "static";
            rest = after;
            continue;
        }
        rest = after;
        break if private { format!("#{word}") } else { word };
    };
    // `foo?()` and `foo!:` say something about the type, not about the shape.
    let rest = rest.trim_start_matches(['?', '!']).trim_start();
    // A generic method writes its parameters after the type parameters.
    let rest = skip_angles(rest);
    let params = rest.strip_prefix('(').map(|inside| {
        let mut depth = 1i32;
        let mut params = String::new();
        for ch in inside.chars() {
            match ch {
                '(' => depth += 1,
                ')' => {
                    depth -= 1;
                    if depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            params.push(ch);
        }
        params
    });
    Some(Member { name, is_static, params })
}

fn is_name_char(c: &char) -> bool { c.is_alphanumeric() || *c == '_' || *c == '$' }

/// Step over a `<..>` type-parameter list, counting nesting so `Map<K, V<T>>`
/// does not end at the first `>`.
fn skip_angles(rest: &str) -> &str {
    let Some(inside) = rest.strip_prefix('<') else { return rest };
    let mut depth = 1i32;
    for (at, ch) in inside.char_indices() {
        match ch {
            '<' => depth += 1,
            '>' => {
                depth -= 1;
                if depth == 0 {
                    return inside[at + 1..].trim_start();
                }
            }
            _ => {}
        }
    }
    rest
}
