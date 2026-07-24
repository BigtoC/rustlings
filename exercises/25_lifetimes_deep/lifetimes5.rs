// Module 1 · Lifetimes (deep) — part 5: which input does the output borrow?
//
// When a function takes several references and returns one, lifetime elision
// CANNOT guess which input the result borrows from, so you must say it
// explicitly. Crucially, not every parameter has to share the output's
// lifetime — only the one the returned reference actually points into.
//
// `first_token` returns a slice of `prefix` (the text before the first
// `separator`). The returned slice therefore borrows `prefix` and has nothing
// to do with how long `separator` lives.

// TODO: Add lifetimes so the compiler knows the return value borrows from
// `prefix` (and NOT from `separator`). Give `prefix` and the return type the
// same lifetime `'a`; leave `separator` with its own, unrelated lifetime:
//   fn first_token<'a>(prefix: &'a str, separator: &str) -> &'a str
fn first_token(prefix: &str, separator: &str) -> &str {
    match prefix.split(separator).next() {
        Some(token) => token,
        None => prefix,
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn returns_a_slice_of_prefix() {
        let text = String::from("key=value");
        let token;
        {
            // `separator` lives only inside this inner scope...
            let separator = String::from("=");
            token = first_token(&text, &separator);
        }
        // ...yet `token` is still valid here, because it borrows `text` (which
        // is still alive), not `separator`. This only compiles if the lifetimes
        // tie the output to `prefix` alone.
        assert_eq!(token, "key");
    }
}
