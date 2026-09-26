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
> This directory holds the `Deref` / `Borrow` / `Cow` part. The `?Sized` part
> (`sized1..3`: unsized types, `?Sized` bounds, and forwarding impls for `&T`
> and `Box<T>`) will be added here later.
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

Related modules: `19_smart_pointers/smart_pointers4` reads `Cow` variants,
`37_borrowck_errors/borrowck2` decides between owned and borrowed returns, and
`60_lru_cache/lru3` applies `borrow1`'s `Borrow<Q>` signature to a cache.
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
