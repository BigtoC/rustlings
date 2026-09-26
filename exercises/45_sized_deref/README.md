# Traits & Abstraction · Deref, Borrow, Cow and ?Sized: Owned Values and Their Borrowed Views

> Part of the "Traits & Abstraction" group, after `44_trait_contracts` and
> before `47_type_level`. Most owning types in std come with a **borrowed
> form**: `String` and `str`, `Vec<T>` and `[T]`, `PathBuf` and `Path`,
> `Box<T>` and `T`. This module is about the traits that connect the two:
> `Deref` makes a wrapper usable as its target, `Borrow` makes owned keys
> searchable by their borrowed form, and `Cow` lets a function return borrowed
> data when it can and owned data when it must. All **std**, **100% safe**,
> **stable** Rust, edition 2024.
>
> The first four exercises are about those traits. The last three
> (`sized1..3`) are about the borrowed forms themselves: `str`, `[T]`, `Path`
> and `dyn Trait` have no size known at compile time, so generic code must
> opt in to them with `?Sized`, pointer types need forwarding impls to pass
> a trait on, and a struct can even end in one.
>
> As in `37_borrowck_errors`, the `// TODO` comments name the error and the
> requirements but **not** the fix, as in an interview. Read what rustc says
> first, and press `h` when you want the full answer.

## Core Ideas

Three std traits hand out a `&U` from a `&T`, and a fourth goes the other
way. They look alike and promise very different things:

| Trait | What it promises | Who uses it |
| --- | --- | --- |
| `Deref<Target = U>` | a `T` transparently behaves like a `U`; exactly one target per type | the compiler: `*v`, deref coercion, method lookup |
| `AsRef<U>` | a cheap reference-to-reference conversion, nothing more | generic parameters such as `P: AsRef<Path>` (`23_conversions/conversions5`) |
| `Borrow<U>` | a reference conversion **plus** a promise: `Hash`, `Eq` and `Ord` give the same answers on the `U` | `HashMap`, `HashSet`, `BTreeMap` lookups |
| `ToOwned` | the way back: build an owned value (`String`) from a borrowed one (`str`) | `.to_owned()`, `Cow` |

`Deref::Target` is an **associated type**, so a type has one deref target
(see `43_assoc_types` for why that choice matters). `AsRef<U>` and `Borrow<U>`
take a **type parameter**, so one type can have many: `String` is `AsRef<str>`,
`AsRef<[u8]>`, `AsRef<OsStr>` and `AsRef<Path>`, but apart from itself it is
`Borrow<str>` only.

The borrowed forms share one more property: `str`, `[T]`, `Path` and
`dyn Trait` are **unsized**. They only exist behind a pointer that also
stores a length or a vtable pointer, and every type parameter refuses them
unless it is declared `?Sized`. That is why `Deref` says
`type Target: ?Sized` and `HashMap::get` says `Q: ?Sized`, and it is the
subject of the second half of the module:

- **The implicit bound.** Every type parameter, `impl Trait` argument and
  associated type is `Sized` unless it opts out; `Self` in a trait is the
  exception (sized1).
- **Forwarding impls.** `Box<dyn Shape>` and `&Circle` are not `Shape` until
  someone writes `impl<S: Shape + ?Sized> Shape for Box<S>` and the same for
  `&S`, as std does for `Display`, `Iterator`, `Read` and `Write` (sized2).
- **Unsized structs.** A struct whose last field is unsized is itself
  unsized, and you get one from a sized value by an unsizing coercion
  (sized3).

## Where Deref Coercion Applies

If `T: Deref<Target = U>`, the compiler converts a `&T` into a `&U` (and
further, `&Box<Username>` to `&Username` to `&str`) only at a **coercion
site**, where the target type is already known:

| Works | Does not work |
| --- | --- |
| function and method arguments: `greet(&name)` with `greet(&str)` | generic parameters: `shout(&name)` with `S: AsRef<str>` needs `Username: AsRef<str>` |
| a `let` with a type: `let s: &str = &name;` | operators: `name == "root"` needs a `PartialEq<&str>` impl |
| struct fields and return values | `match` patterns: a string-literal pattern needs a `&str` scrutinee |
| method calls (auto-deref, not coercion, but the same `deref` calls): `name.len()` searches `Username`, then `str` | trait impls: `Username` gets no `Display` or `Hash` from `str` (so `format!("{name}")` fails while `name.to_string()` works), and `Admin` gets no `Greet` from `User` |
| | inside other types: `&[Username]` never becomes a `&[&str]`, and `Option<&Username>` is not an `Option<&str>` (convert each element with `.map(Deref::deref)`; an `Option<Username>` has `.as_deref()`) |

Where there is no coercion site, deref by hand with `&*name`. Method lookup
stops at the **first** type in the chain that has the method, which is the
`&Box<dyn Shape>` trap in the `32_dispatch` README: a blanket-implemented
`as_any()` resolves to the box's own impl before it reaches the shape.

When to implement `Deref`, by the std docs' rule of thumb: the type
transparently behaves like its target, the deref is cheap and never fails, and
nobody will be surprised by the calls the compiler inserts. The older API
Guidelines are stricter ("only smart pointers implement `Deref`", C-DEREF), so
expect an interviewer to ask which rule you follow for a newtype like
`Username`, and why. `DerefMut` needs more: a type with an invariant (a
validated name, a sorted list) must not hand out a `&mut` to its insides. And
`Deref` is **not inheritance**: an `Admin`
that derefs to `User` gets `User`'s methods, fields and coercions, but none of
its trait impls, so `T: Greet` bounds and `dyn Greet` casts still fail, while
`admin.greet()` quietly runs the `User` version. Compose and delegate instead.
Real smart pointers, for comparison: `deep-dive/src/myarc.rs` (`Deref` only,
because an `Arc` is shared) and `deep-dive/src/raw_vec.rs` (`Deref` and
`DerefMut` to a slice).

## The `Borrow` Contract

```rust
// In `impl<K: Eq + Hash, V, S: BuildHasher> HashMap<K, V, S>`:
pub fn get<Q>(&self, key: &Q) -> Option<&V>
where
    K: Borrow<Q>,
    Q: Hash + Eq + ?Sized,
```

The map hashed each stored key as a `K` and now hashes your query as a `Q`, so
the two must agree. That is the whole contract of `Borrow`, and it decides who
may implement it:

- A newtype `UserId(String)` with a **derived** `Hash` and `Eq` hashes and
  compares exactly like its `str`, so `impl Borrow<str> for UserId` is sound.
- A case-insensitive key must **not** implement `Borrow<str>`: "Alice" and
  "alice" are equal keys with different `str` hashes. Give it `AsRef<str>`
  instead (writing a `Hash` that agrees with such an `Eq` is
  `44_trait_contracts/contracts1`).
- `String` is `AsRef<[u8]>` but not `Borrow<[u8]>`. A `str` hashes its bytes
  and then a `0xff` end marker, a `[u8]` its length and then its bytes, so the
  same bytes give different hashes.
- `Borrow` does not chain. `Rc<String>` is `Borrow<String>` but not
  `Borrow<str>`, so a `HashSet<Rc<String>>` cannot be searched with a `&str`
  (E0277). Use `Rc<str>` (or `Arc<str>`): it is `Borrow<str>`, and it keeps
  the counts and the bytes in one allocation instead of two.

Every type is `Borrow` of itself, which is why `map.get(&owned_key)` keeps
working. It has a side effect worth knowing: when that reflexive impl is the
**only** `Borrow` impl a key type has, rustc infers `Q = K` and reports a
`&str` argument as E0308 "expected `&UserId`, found `&str`", not as a missing
trait impl.

## Returning `Cow`

`Cow<'a, str>` is either `Borrowed(&'a str)` or `Owned(String)`. Return it from
a function that usually hands its input back unchanged: check cheaply whether
anything must change, return `Cow::Borrowed(input)` if not (zero allocations),
and build `Cow::Owned(new_text)` only when needed. A `Cow` derefs to `str`, so
callers barely notice it. `into_owned()` gives a `String` (copying only a
`Borrowed`), and `to_mut()` copies a `Borrowed` on the first write. Spell the
return type `Cow<'_, str>`: since Rust 1.89 the `mismatched_lifetime_syntaxes`
lint warns about `-> Cow<str>`, because the elided lifetime is invisible.
`Cow<'static, str>` is the usual type for messages that are mostly string
literals but sometimes `format!`ed.

## Unsized Types and `?Sized`

A **dynamically sized type** (DST) has no size known at compile time: `str`,
`[T]`, `dyn Trait`, and any struct whose last field is one of those (`Path`,
`OsStr`, `CStr`). You can't keep one in a local, pass one by value or return
one; locals, parameters, `const`s and `static`s must all be `Sized`. A DST
lives behind a pointer that carries the missing piece next to the address:
the length for `str` and `[T]`, a pointer to the vtable for `dyn Trait`.
Those pointers are two words in current rustc, as `41_memory_layout/layout1`
measures (the Reference says not to rely on that).

`Sized` is the marker trait for "the size is known at compile time", and
rustc adds it as a bound almost everywhere:

| Where | Implicitly `Sized`? | How to opt out |
| --- | --- | --- |
| type parameters of functions, structs, enums, impls and traits | yes | `T: ?Sized` or `where T: ?Sized` (sized1, sized3) |
| `impl Trait` in argument position | yes | `&(impl Trait + ?Sized)`: rustc's help leaves out the parentheses, and without them the `+` is "ambiguous" (sized1) |
| associated types (`type Item;`) | yes | `type Target: ?Sized;`, as `Deref` declares it (deref1) |
| `Self` inside a trait | **no** | nothing to opt out of: `impl Display for str` just works |

Because `Self` is `?Sized` in a trait, a provided method that takes `self`
by value needs `where Self: Sized` (E0277 "the size for values of type
`Self` cannot be known"), the same gate that `32_dispatch/dispatch2` uses
for dyn compatibility. The `?` modifier works with `Sized` only ("bound
modifier `?` can only be applied to `Sized`"). And `?Sized` is a promise:
a `T: ?Sized` may only be used behind a pointer, so `fn f<T: ?Sized>(x: T)`
is E0277 again ("the size for values of type `T` cannot be known", with
the help "function arguments must have a statically known size").

The type parameter is the **pointee**. In `fn join_all<T>(items: &[&T])`,
a `&[&str]` argument makes `T = str`, and a `&[&dyn Display]` makes
`T = dyn Display`; the reference around it is part of the signature.

## Forwarding Impls

`Box<dyn Shape>` does not implement `Shape`, although `boxed.area()`
compiles. Method calls auto-deref; trait bounds are checked against the
exact type, as `deref2` showed for `Admin` and `Greet`. The compiler writes
`impl Shape for dyn Shape` itself (for any dyn-compatible trait), but a
`Box<dyn Shape>`, a `&dyn Shape` and a `&Circle` are other types. std closes
that gap for its own traits with one forwarding impl per pointer type:

```rust
// From core, abridged: why `&mut iter` can be passed as an `impl Iterator`.
impl<I: Iterator + ?Sized> Iterator for &mut I {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        (**self).next()
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        (**self).size_hint()
    }
    // ... and `advance_by`, `nth` and more
}
```

- **`?Sized` is the point.** The pointers worth covering are the trait
  objects, `Box<dyn Shape>` and `&dyn Shape`. Without `?Sized` the impl
  covers sized pointees such as `Box<Circle>`, and `Box<dyn Shape>` fails
  with E0277 "the size for values of type `dyn Shape` cannot be known at
  compilation time". A concrete `impl Shape for Box<dyn Shape>` is no
  substitute: `dyn Shape + Send` is another type (as in
  `Box<dyn Error + Send + Sync>`), and only a `?Sized` parameter covers
  every trait object at once.
- **Forward every method, provided ones included.** A method with a default
  body that you don't forward runs the default on the wrapper and ignores
  the pointee's override. That is why std forwards `size_hint` and `nth`
  too, not just `next`.
- **Call the method on the pointee, not on `self`.** Inside
  `impl Shape for Box<S>`, `self.area()` finds this very impl first (method
  lookup tries `&Box<S>` before it derefs), so it calls itself. rustc only
  warns, "function cannot return without recursing", and the program
  overflows its stack.
- **Match the receiver to the pointer.** A trait whose methods take `&self`
  can be forwarded through `&T`, `Box<T>`, `Rc<T>` and `Arc<T>`. A
  `&mut self` method needs `&mut T` or `Box<T>`, which is why std forwards
  `Iterator`, `io::Read` and `io::Write` through `&mut`.
- **It is never automatic**, because forwarding is not always the right
  meaning: `Clone for Box<T>` clones the value into a new box, and
  `impl<E: Error> Error for Box<E>` leaves out `?Sized` on purpose, so that
  `Box<dyn Error>` is not an `Error` (`35_error_design`).
- **A blanket impl over `Deref` is not the same thing.**
  `impl<P: Deref> Shape for P where P::Target: Shape` compiles and covers
  every smart pointer at once, but it is Deref-as-inheritance again
  (`deref2`), and it claims every `Deref` type: a local type that derefs to
  a shape can no longer have a `Shape` impl of its own (E0119, see
  `42_coherence`).

Forwarding through `&T` is also what makes a borrowed test double work. A
service generic over `M: Mailer` that owns its `M` accepts `&mock` once
`impl<T: Mailer + ?Sized> Mailer for &T` exists, so the test keeps the mock
and reads it whenever it likes (`50_testing_seams/seams1` read it back
through the service instead). The Rust API Guidelines ask for the same shape
in I/O code (C-RW-VALUE): take `R: Read` and `W: Write` by value, and let a
caller who wants to keep the reader pass `&mut reader`.

## Unsized Structs

With `struct Packet<T: ?Sized> { id: u32, payload: T }`, a `Packet<[u8]>` is
a DST: an id and then any number of bytes, in one value. std's own DSTs are
built this way (`Path` wraps an `OsStr`, and an `Rc<str>` points at one
allocation that holds both counts and then the bytes).

- **Only the last field** may be unsized ("only the last field of a struct
  may have a dynamically sized type"), since the other fields' offsets must
  be known without reading any metadata.
- **A pointer to the struct carries its last field's metadata.** A
  `&Packet<[u8]>` is two words, and `size_of_val` reads the length from it
  to compute the value's size. For a `&Packet<dyn Display>`, the vtable
  records the payload type's size and alignment, which is enough to compute
  the size of the whole packet.
- **You build a sized one and coerce it.** An unsizing coercion turns a
  `Box<Packet<[u8; 4]>>` into a `Box<Packet<[u8]>>` without moving the
  value; only the pointer gains a length. It is stable for a struct whose
  last field is the only one that involves `T`. Your own smart pointers
  can't join in on stable: the traits behind these coercions, `Unsize` and
  `CoerceUnsized`, are unstable, so a hand-written pointer such as the
  deep-dive `MyArc<T>` (`deep-dive/src/myarc.rs`) never becomes a
  `MyArc<dyn Trait>` the way an `Arc<T>` becomes an `Arc<dyn Trait>`.
- **Each impl block has its own `T`.** `impl<T: ?Sized> Packet<T>` is needed
  for methods on a `Packet<[u8]>`, but a method that takes or returns a `T`
  by value (`new`) needs `T: Sized` back: keep it in a separate `impl<T>`,
  or give it `where T: Sized`.

## Errors You Will Meet in `sized1..3`

| Code | rustc says | Where | What it means |
| --- | --- | --- | --- |
| E0277 | the size for values of type `str` cannot be known at compilation time | sized1 | "required by an implicit `Sized` bound": a type parameter or an `impl Trait` argument that never opted out |
| (none) | ambiguous `+` in a type | sized1, after rustc's help | `&impl AsRef<str> + ?Sized` needs parentheses |
| E0277 | the trait bound `Box<dyn Shape>: Shape` is not satisfied | sized2 | no impl for the pointer type, although method calls auto-deref |
| E0277 | the size for values of type `dyn Shape` cannot be known at compilation time | sized2, a forwarding impl without `?Sized` | the impl covers sized pointees only |
| E0599 | the method `register` exists for struct `SignupService<&MockMailer>`, but its trait bounds were not satisfied | sized2 | the struct's `M: Mailer` bound fails for `&MockMailer` |
| warning | function cannot return without recursing | sized2, `self.area()` in the impl | the call found the wrapper's own method |
| E0277 | the size for values of type `T` cannot be known at compilation time: "only the last field of a struct may have a dynamically sized type" | sized3 | the unsized field has to come last |
| E0599 | the method `id` exists for struct `Box<Packet<[u8]>>`, but its trait bounds were not satisfied | sized3 | the impl block still has its implicit `Sized` bound |
| E0308 | mismatched types: expected `Box<Packet<[u8]>>` | sized3 | no unsizing coercion while `Packet` only takes sized payloads |

## Exercise Path

1. **deref1** — A validated `Username(String)` can't be passed where a `&str`
   is expected (E0308) or asked for `str` methods (E0599). Implement `Deref`
   with `Target = str`, and deliberately no `DerefMut`, so the rules checked by
   `Username::new` can't be broken through a `&mut str`. The tests also show
   the places coercion never reaches (operators, patterns, generic parameters,
   trait impls).
2. **deref2** — `Admin` derefs to `User` to fake inheritance: `admin.greet()`
   compiles but greets as a plain user, and `welcome(&admin)` and the
   `&dyn Greet` / `Box<dyn Greet>` casts fail with E0277. Remove the `Deref`,
   implement `Greet` for `Admin` by delegating to the inner user, and add a
   `user()` accessor.
3. **borrow1** — `lookup(map, key: &K)` forces callers to own a key to search
   for one (E0308 expected `&String`, found `&str`). Give it `HashMap::get`'s
   `K: Borrow<Q>` signature, add `impl Borrow<str> for UserId`, and switch an
   interner from `HashSet<Rc<String>>` (E0277 `Rc<String>: Borrow<str>`) to
   `HashSet<Rc<str>>`.
4. **cow1** — `collapse_spaces` and `normalize` return `String` and allocate on
   every call, and the tests check for `Cow::Borrowed` (E0308). Return
   `Cow<'_, str>`, borrow the input (or a trimmed slice of it) when nothing
   changes, and allocate only when a run of spaces has to go.
5. **sized1** — `join_all<T: Display>(items: &[&T], sep)` rejects `&str` and
   `&dyn Display` items, and `trimmed(text: &impl AsRef<str>)` rejects a
   `&str` (E0277 "the size for values of type `str` cannot be known at
   compilation time"). Relax the implicit `Sized` bound on the function's
   type parameter and on the `impl Trait` argument, so unsized pointees are
   accepted and sized ones keep working.
6. **sized2** — `total_area<S: Shape>` rejects slices of `Box<dyn Shape>`,
   `Box<Square>`, `&Circle` and `&dyn Shape` (E0277), and
   `SignupService::new(&mock)` rejects a borrowed mock (E0277, E0599). Write
   the forwarding impls: `Box<S>` and `&S` for shapes (every trait object
   type included, `dyn Shape + Send` too, and forwarding the provided `name`
   as well), and `&T` for mailers, so a test can lend out a mock that it
   keeps.
7. **sized3** — `Packet<[u8]>` and `Packet<dyn Display>` are rejected
   (E0277, E0599, E0308) because `Packet<T>` takes sized payloads only. Make
   the payload an unsized last field, so that a boxed `Packet<[u8; 4]>`
   unsizes to a `Packet<[u8]>` and one `checksum` serves every length, give
   every packet its `id` and `payload` methods, and keep `new` for sized
   payloads.

Related modules: `19_smart_pointers/smart_pointers4` reads `Cow` variants,
`37_borrowck_errors/borrowck2` decides between owned and borrowed returns, and
`60_lru_cache/lru3` applies `borrow1`'s `Borrow<Q>` signature to a cache.
For the `?Sized` half: `41_memory_layout/layout1` measures the fat pointers,
`32_dispatch` and `deep-dive/src/vtable_lab.rs` show what a vtable holds,
`42_coherence` covers blanket impls and why `Box` counts as local, and
`50_testing_seams/seams1` builds the recording mock that `sized2` lends out.
The tests in `deref1` and `deref2` spot a stray `Deref` or `DerefMut` impl
with a run-time probe that works only on concrete types. The stricter check,
that `name.make_ascii_uppercase()` does **not** compile, needs a
`compile_fail` doctest; that is the planned `api-surface-lab`
(`deep-dive/src/api_surface.rs`).

## Further Reading

- [`std::ops::Deref`](https://doc.rust-lang.org/std/ops/trait.Deref.html), especially "When to implement `Deref` or `DerefMut`"
- [Treating Smart Pointers Like Regular References (The Book)](https://doc.rust-lang.org/book/ch15-02-deref.html)
- [Type coercions: coercion sites (The Reference)](https://doc.rust-lang.org/reference/type-coercions.html#coercion-sites)
- [Method-call expressions (The Reference)](https://doc.rust-lang.org/reference/expressions/method-call-expr.html)
- [`std::borrow::Borrow`](https://doc.rust-lang.org/std/borrow/trait.Borrow.html), with the case-insensitive string example
- [`std::convert::AsRef`](https://doc.rust-lang.org/std/convert/trait.AsRef.html)
- [`HashMap::get`](https://doc.rust-lang.org/std/collections/struct.HashMap.html#method.get)
- [`std::borrow::Cow`](https://doc.rust-lang.org/std/borrow/enum.Cow.html)
- [Rust API Guidelines: only smart pointers implement `Deref` and `DerefMut` (C-DEREF)](https://rust-lang.github.io/api-guidelines/predictability.html#c-deref)
- [Rust Design Patterns: `Deref` polymorphism, an anti-pattern](https://rust-unofficial.github.io/patterns/anti_patterns/deref.html)
- [The `mismatched_lifetime_syntaxes` lint](https://doc.rust-lang.org/rustc/lints/listing/warn-by-default.html#mismatched-lifetime-syntaxes)
- [`std::marker::Sized`](https://doc.rust-lang.org/std/marker/trait.Sized.html)
- [Dynamically sized types (The Reference)](https://doc.rust-lang.org/reference/dynamically-sized-types.html) and [`?Sized` bounds (The Reference)](https://doc.rust-lang.org/reference/trait-bounds.html#sized)
- [Dynamically Sized Types and the `Sized` Trait (The Book)](https://doc.rust-lang.org/book/ch20-03-advanced-types.html#dynamically-sized-types-and-the-sized-trait)
- [Exotically Sized Types: DSTs (The Nomicon)](https://doc.rust-lang.org/nomicon/exotic-sizes.html#dynamically-sized-types-dsts)
- [Unsized coercions (The Reference)](https://doc.rust-lang.org/reference/type-coercions.html#unsized-coercions), and the unstable traits behind them, [`Unsize`](https://doc.rust-lang.org/std/marker/trait.Unsize.html) and [`CoerceUnsized`](https://doc.rust-lang.org/std/ops/trait.CoerceUnsized.html)
- [`impl<I: Iterator + ?Sized> Iterator for &mut I`](https://doc.rust-lang.org/std/iter/trait.Iterator.html#impl-Iterator-for-%26mut+I), a forwarding impl in std
- [Rust API Guidelines: generic reader/writer functions take `R: Read` and `W: Write` by value (C-RW-VALUE)](https://rust-lang.github.io/api-guidelines/interoperability.html#c-rw-value)
- [`std::mem::size_of_val`](https://doc.rust-lang.org/std/mem/fn.size_of_val.html) and [`std::ptr::addr_eq`](https://doc.rust-lang.org/std/ptr/fn.addr_eq.html)
