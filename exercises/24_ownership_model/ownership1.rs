// Module 1 · Ownership — part 1: MOVE semantics and use-after-move.
//
// Passing a value to a function BY VALUE *moves* it: ownership is transferred
// into the callee, and the original binding can no longer be used. If the
// caller still needs the value afterwards, moving is the wrong choice — you
// either BORROW it (`&T`, when you only need to read/inspect) or CLONE it
// (`.clone()`, when you truly need a second independent owner).
//
// Counting characters only needs to READ the string, so borrowing is the right,
// zero-cost answer here.

// TODO: This helper takes its argument BY VALUE, which MOVES the `String` into
// it and destroys it at the end of the call. The caller below still needs the
// string afterwards, so this signature is wrong. Change it to BORROW the text
// (take `&str`) so the caller keeps ownership.
fn count_chars(text: String) -> usize {
    text.chars().count()
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_is_still_usable_after_counting() {
        let text = String::from("rustaceans");
        // TODO: Pass a borrow (`&text`) instead of moving `text` away.
        let count = count_chars(text);
        assert_eq!(count, 10);
        // Because counting only *borrows*, `text` is still ours to use here.
        assert_eq!(text, "rustaceans");
    }
}
