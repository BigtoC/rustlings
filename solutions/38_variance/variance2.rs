// Module 1 · Variance and PhantomData — part 2: `&'a mut self` on a `Foo<'a>` borrows it for good, and so does a `&'a mut &'a str` field (E0499, E0502, E0621).
//
// Part 1 was about choosing a type's variance. This part is about the one
// invariance you meet every week: `&'b mut T` is covariant in `'b` but
// INVARIANT in `T`. It has to be. If a `&mut Vec<&'static str>` could be used
// as a `&mut Vec<&'a str>`, you could push a short-lived `&'a str` through it,
// and the owner of the `Vec` would later read that entry as a `&'static str`,
// long after the text was freed. So behind a `&mut`, every lifetime inside
// `T` is frozen: it must match exactly, and the compiler may no longer shrink
// it to make a borrow fit.
//
// Part A: `Interner<'a>` stores `&'a str`s that borrow the caller's text and
// hands out small integer symbols for them. Its `add` is declared as
//
//     impl<'a> Interner<'a> {
//         fn add(&'a mut self, word: &'a str) -> usize
//     }
//
// It LOOKS tidy: "`'a` is the interner's lifetime, so use it everywhere". But
// it reuses the struct's lifetime parameter for the borrow of `self`. Your
// `interner` has some type `Interner<'x>`, and `'x` must cover every later use
// of it. The call asks for a `&'a mut Interner<'a>`. Behind the `&mut`, the
// `Interner<'x>` cannot be shrunk (invariance), so `'a` has to BE `'x`, and
// the call borrows `interner` mutably for all of `'x`: from the first `add`
// to the last use of the interner. The second `add` is then E0499 and `len()`
// is E0502, even though `add` returns a plain `usize` and nothing you wrote
// holds on to the borrow. (Compare `37_borrowck_errors/borrowck4`, where the
// RETURNED `&str` kept a `&mut` alive. Here the signature alone does it.) The
// tell-tale sign is rustc saying "first borrow later used here" at the very
// call that conflicts: no code of yours uses the first borrow, the TYPE does.
//
// Why doesn't `&'a self` cause the same trouble? `&T` is covariant in `T`, so
// for one call the compiler may view the `Interner<'x>` as an
// `Interner<'short>`. `&'a self` on a `Foo<'a>` is usually harmless; `&'a mut
// self` on a `Foo<'a>` is almost always a bug.
//
// The helper `intern_all` shows how the bug spreads. It holds a
// `&mut Interner<'a>` for a short while only, so it cannot supply the long
// `&'a mut` that `add` demands: E0621 "explicit lifetime required in the type
// of `interner`", whose help line suggests `interner: &'a mut Interner<'a>`.
// Don't take it. The E0621 goes away, but the E0499 next to it stays: every
// pass of the loop still borrows `*interner` for all of `'a`, so the second
// word collides with the first ("mutably borrowed here in the previous
// iteration of the loop"). On top of that, every caller of `intern_all` now
// lends its interner for good. The suggestion treats the symptom in front of
// you; the cause is the signature of `add`.
//
// Part B: the same trap in a field. `WordCursor` walks the caller's text word
// by word by advancing the caller's own `&str` in place, so the caller can
// stop, look at what is left, and carry on. That is a real pattern:
// `impl Read for &[u8]` advances the slice it reads from, and parser
// libraries such as `winnow` take their input as `&mut &str`. The field
//
//     input: &'a mut &'a str
//
// uses ONE lifetime for two different things: how long the cursor may write
// to the caller's `rest` variable (a short borrow) and how long the text lives
// (as long as the words you get back must stay valid). Invariance glues the
// two together: the `rest: &'r str` behind the `&mut` must be exactly a
// `&'a str`, so `rest` stays mutably borrowed for as long as its own lifetime
// reaches, and reading `rest` afterwards is E0502.
//
// How interviewers probe this: "Why does the second `add` fail when `add`
// returns nothing that borrows? Why is `&mut T` invariant, and what would
// break if it were not? Why is `&'a self` fine but `&'a mut self` not? Would
// you apply rustc's `&'a mut` suggestion?"

// ---------- Part A: an interner that locks itself ----------

struct Interner<'a> {
    words: Vec<&'a str>,
}

impl<'a> Interner<'a> {
    fn new() -> Self {
        Interner { words: Vec::new() }
    }

    // `&mut self` gets its own, elided lifetime: this is
    // `fn add<'b>(&'b mut self, word: &'a str)`. The `&'b mut Interner<'a>` is
    // still invariant, so `'a` must still be exactly the interner's lifetime,
    // but `'b` is a separate, fresh borrow that ends when the call returns.
    // `'a` stays only where it belongs, on the stored `word`, which must
    // outlive the interner because `words` keeps it.
    fn add(&mut self, word: &'a str) -> usize {
        if let Some(symbol) = self.words.iter().position(|w| *w == word) {
            return symbol;
        }
        self.words.push(word);
        self.words.len() - 1
    }

    fn len(&self) -> usize {
        self.words.len()
    }

    // The stored word keeps the TEXT's lifetime `'a`, not the lifetime of
    // this `&self` borrow, so it may outlive the interner.
    fn get(&self, symbol: usize) -> Option<&'a str> {
        self.words.get(symbol).copied()
    }

    fn into_words(self) -> Vec<&'a str> {
        self.words
    }
}

// Interns every whitespace-separated word of `text` and returns the symbols in
// order. Don't change this function.
fn intern_all<'a>(interner: &mut Interner<'a>, text: &'a str) -> Vec<usize> {
    let mut symbols = Vec::new();
    for word in text.split_whitespace() {
        symbols.push(interner.add(word));
    }
    symbols
}

// ---------- Part B: a cursor over the caller's `&str` ----------

// Two lifetimes for two borrows: `'s` is how long this cursor may write to
// the caller's `rest` (short, one pass), `'a` is how long the text lives. The
// `&mut` is still invariant in `&'a str`, but `'a` is now the text's own
// lifetime, which nobody needs to shrink, and the mutable borrow `'s` is free
// to end as soon as the cursor is dropped. `next_word` copies the `&'a str`
// out of `*self.input`, so the words borrow the text, never `rest`.
struct WordCursor<'s, 'a> {
    input: &'s mut &'a str,
}

impl<'s, 'a> WordCursor<'s, 'a> {
    fn new(input: &'s mut &'a str) -> Self {
        WordCursor { input }
    }

    // Skips leading whitespace, returns the next word and advances the
    // caller's `&str` to just after it. `None` (with the input advanced to
    // `""`) once only whitespace is left.
    fn next_word(&mut self) -> Option<&'a str> {
        let trimmed = self.input.trim_start();
        let end = trimmed.find(char::is_whitespace).unwrap_or(trimmed.len());
        let (word, tail) = trimmed.split_at(end);
        *self.input = tail;
        if word.is_empty() { None } else { Some(word) }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------- Part A ----------

    #[test]
    fn three_adds_then_len() {
        let text = String::from("hello big world");
        let mut interner = Interner::new();
        let hello = interner.add(&text[0..5]);
        let big = interner.add(&text[6..9]);
        let world = interner.add(&text[10..]);
        assert_eq!(interner.len(), 3);
        assert_eq!((hello, big, world), (0, 1, 2));
    }

    #[test]
    fn repeated_words_keep_their_first_symbol() {
        let text = String::from("to be or not to be");
        let mut interner = Interner::new();
        assert_eq!(intern_all(&mut interner, &text), [0, 1, 2, 3, 0, 1]);
        assert_eq!(interner.len(), 4);
        // `intern_all` can be called again on the same interner...
        assert_eq!(intern_all(&mut interner, "be a question"), [1, 4, 5]);
        assert_eq!(interner.len(), 6);
        assert_eq!(interner.get(5), Some("question"));
        assert_eq!(interner.get(6), None);
        // ...and so can `add`.
        assert_eq!(interner.add(&text[3..5]), 1); // "be"
    }

    #[test]
    fn stored_words_are_the_callers_text() {
        let text = String::from("alpha beta gamma");
        let mut interner = Interner::new();
        intern_all(&mut interner, &text);
        // "beta gamma" again: nothing new.
        intern_all(&mut interner, &text[6..]);
        // The text stays readable while the interner borrows it.
        assert!(text.starts_with("alpha"));

        let words = interner.into_words();
        assert_eq!(words, ["alpha", "beta", "gamma"]);
        // The caller's own bytes, not copies.
        assert_eq!(words[0].as_ptr(), text.as_ptr());
        assert_eq!(words[1].as_ptr(), text[6..].as_ptr());
        assert_eq!(words[2].as_ptr(), text[11..].as_ptr());
    }

    #[test]
    fn a_word_outlives_the_interner() {
        let text = String::from("left right");
        let right = {
            let mut interner = Interner::new();
            intern_all(&mut interner, &text);
            interner.get(1)
        };
        assert_eq!(right, Some("right"));
        assert_eq!(right.map(str::as_ptr), Some(text[5..].as_ptr()));
    }

    // Adding a word takes `&mut self`, so the interner needs no interior
    // mutability: like the `Vec` inside it, it can be shared between threads.
    fn assert_sync<T: Sync>(_: &T) {}

    #[test]
    fn a_filled_interner_can_be_shared() {
        let text = String::from("x y x");
        let mut interner = Interner::new();
        assert_eq!(intern_all(&mut interner, &text), [0, 1, 0]);
        assert_sync(&interner);
        assert_eq!(interner.get(1), Some("y"));
    }

    // ---------- Part B ----------

    #[test]
    fn the_cursor_yields_the_words_in_order() {
        let text = String::from("hello big world");
        let mut rest: &str = &text;
        let mut cursor = WordCursor::new(&mut rest);
        let words = (cursor.next_word(), cursor.next_word(), cursor.next_word());
        assert_eq!(cursor.next_word(), None);
        assert_eq!(rest, "");
        assert_eq!(words, (Some("hello"), Some("big"), Some("world")));
    }

    #[test]
    fn the_caller_reads_its_rest_between_passes() {
        let text = String::from("  one two  three ");
        let mut rest: &str = &text;

        let (first, second) = {
            let mut cursor = WordCursor::new(&mut rest);
            (cursor.next_word(), cursor.next_word())
        };
        assert_eq!(rest, "  three ");

        // A new cursor picks up where the last one stopped.
        let third = WordCursor::new(&mut rest).next_word();

        // The words borrow `text`, not `rest` or a cursor: all still usable.
        assert_eq!(
            (first, second, third),
            (Some("one"), Some("two"), Some("three"))
        );
        assert_eq!(first.map(str::as_ptr), Some(text[2..].as_ptr()));
        assert_eq!(third.map(str::as_ptr), Some(text[11..].as_ptr()));
    }

    #[test]
    fn empty_and_blank_input_yield_nothing() {
        let mut empty = "";
        assert_eq!(WordCursor::new(&mut empty).next_word(), None);
        let mut blank = " \t\n ";
        assert_eq!(WordCursor::new(&mut blank).next_word(), None);
    }
}
