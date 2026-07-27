// Module 1 · Lifetimes (deep) — part 4: a struct that holds a reference.
//
// A struct that stores a reference does NOT own the data it points at. The
// compiler must be told that the struct may not outlive whatever it borrows —
// otherwise the reference could dangle. We express that constraint with a
// lifetime parameter on the struct: `struct Excerpt<'a> { part: &'a str }`
// reads as "an `Excerpt<'a>` is valid only for as long as the `&'a str` it
// holds is valid".

struct Excerpt<'a> {
    part: &'a str,
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
        // `split_once` genuinely can fail, so this `expect` means something —
        // `split('.').next()` is always `Some`, which would make it theatre.
        let (first_sentence, _) = novel.split_once('.').expect("no '.' in the text");
        let excerpt = Excerpt {
            part: first_sentence,
        };
        assert_eq!(excerpt.part, "Call me Ishmael");
    }
}
