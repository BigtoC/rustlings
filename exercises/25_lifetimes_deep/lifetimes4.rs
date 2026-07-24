// Module 1 · Lifetimes (deep) — part 4: a struct that holds a reference.
//
// A struct that stores a reference does NOT own the data it points at. The
// compiler must be told that the struct may not outlive whatever it borrows —
// otherwise the reference could dangle. We express that constraint with a
// lifetime parameter on the struct: `struct Excerpt<'a> { part: &'a str }`
// reads as "an `Excerpt<'a>` is valid only for as long as the `&'a str` it
// holds is valid".

// TODO: This struct holds a borrowed string slice but declares no lifetime, so
// it won't compile. Add a lifetime parameter `'a` to the struct and tie the
// `part` field to it: `struct Excerpt<'a>` with `part: &'a str`.
struct Excerpt {
    part: &str,
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_a_slice_of_the_source() {
        let novel = String::from("Call me Ishmael. Some years ago...");
        let first_sentence = novel.split('.').next().expect("no '.' in the text");
        let excerpt = Excerpt {
            part: first_sentence,
        };
        assert_eq!(excerpt.part, "Call me Ishmael");
    }
}
