// Module 1 · Ownership — part 2: COPY vs MOVE.
//
// Some types are `Copy`: assigning or passing them by value duplicates the bits
// and leaves the original usable. This is only allowed for types that are cheap
// and safe to bit-copy — small aggregates of `Copy` fields (integers, `bool`,
// `char`, ...). Types that own a heap allocation (`String`, `Vec<T>`, ...) are
// NOT `Copy`: passing them by value MOVES them, and the original is gone. When
// you need a second independent owner of such a type, you call `.clone()`.
//
// A `Point` here is just two integers, so it is eligible to be `Copy`.

// TODO: Make `Point` a `Copy` type. Add `Clone` and `Copy` to the derive list
// so that `let q = p;` COPIES the point instead of moving it (Rust requires
// `Clone` alongside `Copy`).
#[derive(Debug, PartialEq)]
struct Point {
    x: i32,
    y: i32,
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn copy_type_survives_being_assigned_away() {
        let p = Point { x: 1, y: 2 };
        // With `Copy`, this duplicates `p` instead of moving it.
        let q = p;
        // ...so both `p` and `q` are still valid afterwards.
        assert_eq!(p, Point { x: 1, y: 2 });
        assert_eq!(q, Point { x: 1, y: 2 });
    }

    #[test]
    fn non_copy_type_must_be_cloned() {
        // A `String` owns a heap buffer, so it is NOT `Copy`. Moving it would
        // invalidate the original; when we want two owners we `.clone()`.
        let original = String::from("hello");
        let duplicate = original.clone();
        assert_eq!(original, "hello");
        assert_eq!(duplicate, "hello");
    }
}
