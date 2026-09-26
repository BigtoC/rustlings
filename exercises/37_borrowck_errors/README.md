# Module 1 · Borrow-Checker Errors: Read the Error, Then Fix the Design

> The interview-drill companion to `24_ownership_model` and `25_lifetimes_deep`.
> Every exercise reproduces a borrow-checker error that Rust interviews put on
> the whiteboard, and you fix it by **restructuring** the code, not by reaching
> for `.clone()`, `RefCell` or `unsafe`. All **std**, **100% safe**, **stable**
> Rust, edition 2024.
>
> Unlike earlier modules, the `// TODO` comments name the error and the
> requirements but **not** the fix, as in an interview. Read what rustc says
> first. Press `h` when you want the full answer.

## Core Ideas

Every error here is one of two rules, seen from a different angle:

- **Aliasing XOR mutability.** A place has either any number of shared `&T`
  borrows or exactly one exclusive `&mut T`. With NLL a borrow lives until its
  last use, so an error means two borrows really do overlap in time.
- **No reference outlives its owner.** A temporary dies at the end of its
  statement (unless its lifetime is extended), and a local dies when the
  function returns.

The borrow checker works **one function at a time** and trusts only
**signatures**. That explains most of the surprises:

- `&mut v[i]` and `&mut v[j]` conflict because the checker never evaluates
  `i` or `j`; both are borrows of the place `v[_]`.
- A `&mut self` method borrows the **whole** struct, whatever field its body
  touches. Direct field borrows are disjoint (a *split borrow*).
- A reference returned from a `&mut self` method keeps the object
  **exclusively** borrowed for as long as the reference lives. There is no
  "downgrade" to a shared borrow, and that is deliberate: it would be unsound.
- A borrow that *might* be returned must last for the caller's lifetime, so NLL
  treats it as live on every path through the function, including the path
  that does not return it (NLL "problem case #3").
- `&mut T` is not `Copy`. Passing it where the expected type is `&mut _`
  reborrows it implicitly (`f(&mut *x)`). Passing it to a bare generic `W`
  moves it.

## Reading the Error

| Code  | rustc says                                                               | What it means                                                      | Idiomatic fixes                                                                      |
| ----- | ------------------------------------------------------------------------ | ------------------------------------------------------------------ | ------------------------------------------------------------------------------------ |
| E0499 | cannot borrow `x` as mutable more than once at a time                    | two live `&mut` the checker cannot prove disjoint                  | `split_at_mut`, `get_disjoint_mut`, field borrows, entry API, look up an index first |
| E0502 | cannot borrow `x` as immutable because it is also borrowed as mutable    | a shared borrow while an exclusive one (or anything from it) lives | finish the `&mut` first, separate mutation from reads, copy a `Copy` value out       |
| E0716 | temporary value dropped while borrowed                                   | you kept a borrow of an unnamed temporary past its statement       | bind the value with `let`                                                            |
| E0515 | cannot return reference to local variable / value referencing local data | the function returns a borrow of something it owns                 | return an owned value, borrow from an input, `Cow`, write into a caller's buffer     |
| E0382 | use of moved value                                                       | a non-`Copy` value (including a `&mut`) was moved earlier          | borrow instead of moving; for a `&mut`, reborrow with `&mut *x`                      |

Related errors covered elsewhere: E0597 "does not live long enough"
(`25_lifetimes_deep`), E0373 "closure may outlive the current function"
(`33_closures`), and the run-time version of E0499, `RefCell`'s
`BorrowMutError` (`31_debugging/debugging4`).

## Exercise Path

1. **borrowck1** — Two E0499s. Two `&mut tickets[_]` into one slice: get
   borrows the checker can prove disjoint with `get_disjoint_mut` or
   `split_at_mut`, and return errors for bad indices instead of panicking. Then
   two `&mut self` accessor calls on one struct: borrow the two fields directly
   (or add one splitting method).
2. **borrowck2** — E0716 and E0515. A `Vec<&str>` points into a temporary
   `String` that dies at the end of its statement: give the temporary a name.
   Two functions return borrows of their own locals: return an owned `String`
   where the text is new, and borrow from the input where the answer is already
   part of it.
3. **borrowck3** — NLL problem case #3 (E0499). A get-or-insert on a `HashMap`
   and a find-or-push on a `Vec` are correct code that NLL still rejects. Use
   the entry API (one lookup) for the map, and look up an index first for the
   `Vec`. Polonius accepts both originals, but it is nightly-only.
4. **borrowck4** — E0502 and E0382. The `&str` returned by `push(&mut self)`
   keeps the whole log exclusively borrowed: redesign the API so mutating and
   reading are separate calls. Then a `&mut String` passed to a generic
   `W: Write` is moved rather than reborrowed: reborrow it explicitly.

`38_variance/variance2` comes next: a `&'a mut self` that keeps a struct
borrowed for good. Later, `quizzes/quiz4` (after `47_type_level`) asks you to
predict rustc's verdict on short snippets, two-phase borrows included.

## Further Reading

- [References and Borrowing (The Book)](https://doc.rust-lang.org/book/ch04-02-references-and-borrowing.html)
- [RFC 2094: non-lexical lifetimes](https://rust-lang.github.io/rfcs/2094-nll.html), especially "Problem case #3" and "Layer 4: Named lifetimes"
- [Temporary lifetime extension (The Reference)](https://doc.rust-lang.org/reference/destructors.html#temporary-lifetime-extension)
- [`slice::get_disjoint_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.get_disjoint_mut) and [`slice::split_at_mut`](https://doc.rust-lang.org/std/primitive.slice.html#method.split_at_mut)
- [The `HashMap` entry API](https://doc.rust-lang.org/std/collections/hash_map/enum.Entry.html)
- [Polonius, the next-generation borrow checker](https://rust-lang.github.io/polonius/)
- [Rust API Guidelines: C-RW-VALUE](https://rust-lang.github.io/api-guidelines/interoperability.html#generic-readerwriter-functions-take-r-read-and-w-write-by-value-c-rw-value)
- [Common Rust Lifetime Misconceptions](https://github.com/pretzelhammer/rust-blog/blob/master/posts/common-rust-lifetime-misconceptions.md)
- [Rust error code index](https://doc.rust-lang.org/error_codes/error-index.html)
