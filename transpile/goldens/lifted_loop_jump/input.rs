// A jump written inside a LIFTED body belongs to whichever loop it names. The
// `continue` here names the `for` written in the arm, so it is an ordinary
// `continue`; handed back to the arm's caller it left the arm on the first NUL
// byte, and ankql's `generate_expr_sql` wrote an unterminated SQL literal.
// `break 'rows` names a loop outside the lift, so that one still travels out.
pub enum Refusal {
    Empty,
}

pub enum Lit {
    Text(String),
    Count(u32),
}

pub fn quote(lit: &Lit, out: &mut String) -> Result<usize, Refusal> {
    match lit {
        Lit::Text(s) => {
            out.push('\'');
            for c in s.chars() {
                if c == '\0' {
                    continue;
                }
                out.push(c);
            }
            out.push('\'');
        }
        Lit::Count(n) => {
            if *n == 0 {
                return Err(Refusal::Empty);
            }
            out.push('n');
        }
    }
    Ok(out.len())
}

pub fn quote_all(lits: &Vec<Lit>, out: &mut String) -> Result<usize, Refusal> {
    'rows: for lit in lits {
        match lit {
            Lit::Text(s) => {
                for c in s.chars() {
                    if c == '!' {
                        break 'rows;
                    }
                    out.push(c);
                }
            }
            Lit::Count(n) => {
                if *n == 0 {
                    return Err(Refusal::Empty);
                }
                out.push('n');
            }
        }
    }
    Ok(out.len())
}
