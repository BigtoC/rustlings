# config-lab · serde at the boundary and additive Cargo features

> A deep-dive lab in a crate of its own: a finished reference implementation
> to read, run and break, not a graded exercise. It belongs with the
> "Traits & Abstraction" group, after `35_error_design` and `47_type_level`.
> It needs crates.io crates (serde, toml, proptest) and Cargo features, which
> the single-file graded exercises cannot declare, so it is excluded from the
> rustlings workspace, and CI checks it in a job of its own, once per feature
> set. Stable Rust, edition 2024, `unsafe_code = "forbid"`.

A service reads its configuration from a TOML file. This crate turns that
untrusted text into types the rest of the program can trust. It also compiles
in three shapes: with the `serde` feature (the default), without it, and with
`lenient`, a deliberately bad feature that shows why features may only ever
add things.

| Part | Files | What it reveals |
| --- | --- | --- |
| `serde_boundary` | `src/serde_boundary.rs`, `tests/serde_boundary.rs`, `tests/email.rs` | An `Email` newtype that serde can only build through `TryFrom<String>`; `deny_unknown_fields`, `rename_all` and defaults; a proptest round trip |
| `features_and_cfg` | `Cargo.toml`, `build.rs`, `src/features_and_cfg.rs`, `examples/report.rs` | An optional `serde` feature gated with `cfg_attr`; a `build.rs` cfg declared with `rustc-check-cfg` and rerun with `rerun-if-env-changed` |

## The sharpest question

"How do you guarantee that a deserialized `Email` field is always valid, and
why must Cargo features be additive?"

- **Make the checked constructor the only way in, and make serde use it.**
  `Email(String)` has a private field and one fallible constructor,
  `TryFrom<String>` (`FromStr` calls it). `#[serde(try_from = "String")]`
  makes the derived `Deserialize` read a `String` and pass it to
  `Email::try_from`, turning an `Err` into a deserialization error with
  `serde::de::Error::custom`. A plain `#[derive(Deserialize)]` would not call
  your constructor at all: the generated code lives inside your crate, so the
  private field does not stop it from building `Email(any_string)`
  (`UncheckedEmail` demonstrates it). Past the boundary, code takes `&Email`
  and never checks again ("parse, don't validate").
- **Cargo builds one copy of a dependency for all the crates in the build
  that use it, with the union of the features they asked for.** This is
  feature unification. If crate A relies on `config-lab` rejecting typos and
  crate B, anywhere in the same build, enables `lenient`, then A gets the
  lenient build, and nothing in A's own `Cargo.toml` says so. So a feature may
  add API and behavior, but never remove or change it: whatever compiles and
  works with a feature off must compile and work the same with it on.

## Part 1 · `serde_boundary`

`src/serde_boundary.rs` defines `Email`, `ServiceConfig` and its `[server]`
table, `ServerConfig`:

```toml
name = "billing"
admin-email = "ops@example.com"
alert-emails = ["oncall@example.com"]   # optional, default []

[server]                                # optional, and so is every key in it
port = 9000                             # also: host, max-connections, log-level
```

| Attribute | On | What it does |
| --- | --- | --- |
| `try_from = "String"` | `Email` | Deserialize a `String`, then call `Email::try_from` |
| `into = "String"` | `Email` | Serialize through `From<Email> for String`; serde clones the value first, so the type must be `Clone` |
| `deny_unknown_fields` | both structs | An unknown (misspelled) key is an error that lists the keys that exist |
| `rename_all = "kebab-case"` | both structs | `admin_email` in Rust is `admin-email` in the file, in both directions |
| `default` on a field | `alert_emails`, `server` | A missing key takes the field type's `Default` |
| `default` on a struct | `ServerConfig` | Each missing key takes its value from `ServerConfig::default()` |

The invariants:

- An `Email` holds exactly one `@`, a valid local part, and a domain in
  lowercase (domains are case-insensitive, so `Ops@Example.COM` becomes
  `Ops@example.com`). Checking an `Email` a second time gives the same value.
- Every key in the file is one the code knows.
- A missing optional key means its documented default, never a guess:
  `ServerConfig::default()` is written by hand, because a derived one would
  give port 0 (as in `47_type_level/builder1`).
- All of the above holds without the `serde` feature too: every serde
  attribute sits inside a `cfg_attr` whose condition requires
  `feature = "serde"`, so only the TOML functions (and the `UncheckedEmail`
  demonstration, a `#[cfg(feature = "serde")]` item) disappear.

What the tests pin down (`tests/serde_boundary.rs`, messages as toml 1.x
reports them through `message()`):

| Input | Result |
| --- | --- |
| `admin-email = "ops.example.com"` | `invalid email "ops.example.com": missing '@'`, with the span on the value |
| `prot = 9000` under `[server]` | ``unknown field `prot`, expected one of `host`, `port`, `max-connections`, `log-level` `` |
| `alert-email = [..]` (singular) | ``unknown field `alert-email`, expected one of ..`` |
| no `admin-email` | ``missing field `admin-email` `` |
| `log-level = "verbose"` | ``unknown variant `verbose`, expected one of `debug`, `info`, `warn`, `error` `` |
| `port = 70000` | ``invalid value: integer `70000`, expected u16`` |
| any `ServiceConfig` (proptest) | written with `to_toml_string` and parsed back unchanged |

Details worth knowing:

- **Why the typo test matters.** Both typos above are keys with defaults.
  Without `deny_unknown_fields` serde skips the unknown key, the default
  applies, and the service quietly listens on port 8080 or pages nobody. A
  typo in a required key fails anyway ("missing field"), so a typo test
  built on a required key passes even without the attribute and proves
  nothing.
- **`TryFrom<String>`, not `TryFrom<&str>`.** Taking the `String` by value
  stores a valid address without copying it (a test compares the pointers),
  and hands a rejected one back through `EmailError::into_input`, the way
  `String::from_utf8` returns the bytes in its error.
- **serde keeps only the message.** `de::Error::custom` takes anything that
  implements `Display`, and toml stores the text, so the `EmailError` value
  (and its `kind()`) does not survive deserialization. Callers who must
  branch on the reason call `Email::try_from` themselves.
- **toml points at the whole array** when one element of `alert-emails` is
  invalid, not at the element (tested).
- **`into` costs a clone.** serde calls `Into<String>` on a clone of the
  `Email`. A hand-written `Serialize` that calls
  `serializer.serialize_str(self.as_str())` avoids it.
- **`deny_unknown_fields` and `#[serde(flatten)]` don't mix.** The serde
  docs call the combination unsupported. With serde 1.0.229 the attribute
  still works on the outer struct (the message no longer lists the expected
  keys), and on a struct that is flattened into another it silently does
  nothing.
- **A round trip cannot catch missing validation.** It only feeds back what
  the program wrote itself, and the program only writes valid values. Delete
  `try_from` from `Email` and the round-trip property still passes, while
  the five tests that feed in invalid or unnormalized input fail (see Try
  it).

## Part 2 · `features_and_cfg`

### Why features must be additive

```text
app ──► config-lab                   default features: typos are errors
 └───► plugin ──► config-lab         features = ["lenient"]

Cargo builds ONE config-lab for both, with the union: default + lenient.
app never asked for lenient, yet its config typos are now ignored.
```

- **A feature adds** an optional dependency, a module, an impl or a function.
  `serde` here is additive: it adds `Serialize` / `Deserialize` impls and
  `ServiceConfig::from_toml_str` / `to_toml_string`, and everything that
  exists without it works the same with it.
- **`cfg(not(feature = ".."))` is the smell.** It means that turning the
  feature on takes something away. `lenient` does exactly that to
  `deny_unknown_fields`; the additive way to offer leniency is an API the
  caller chooses (a second function, or a runtime option), which affects
  only that caller.
- **Mutually exclusive features break the same way**: two crates that pick
  different "backends" in one build get both. Prefer a runtime choice or
  separate crates.
- **`default-features = false` is not a guarantee.** It only means "I don't
  need them"; if any other crate in the build uses the defaults, they are on
  for everyone (verified: a crate with `default-features = false` still sees
  `serde` when built together with one that did not opt out).
- **The version 2 feature resolver** (the default since edition 2021;
  edition 2024's resolver 3 keeps its feature rules) narrows unification,
  and one of its rules surprises people: features that only dev-dependencies
  ask for are enabled only when building a target that uses them. Verified
  with a crate whose dev-dependency enables `lenient`: `cargo run` rejects
  the typo, while `cargo test` and `cargo run --example` accept it. Build
  dependencies and proc macros do not share features with normal
  dependencies, so Cargo compiles a second copy when they ask for different
  ones: with `lenient` requested only through a build dependency, `build.rs`
  sees `lenient` and the program itself still rejects typos.
- **`dep:` hides an optional dependency's implicit feature.**
  `serde = ["dep:serde", "dep:toml"]` lets the feature be called `serde`, and
  `cargo build --features toml` is an error: the package "does not have
  feature `toml`".
- **`required-features`** on the `serde_boundary` test target makes
  `cargo test --no-default-features` skip it instead of failing to build it.

### `cfg(feature)`, `cfg_attr` and build-script cfgs

| | `#[cfg(feature = "x")]` | `#[cfg_attr(feature = "x", attr)]` | build-script cfg |
| --- | --- | --- | --- |
| What it does | keeps or removes an item | keeps the item, adds `attr` only when on | keeps or removes an item |
| Who decides | the crates that depend on this one | the same | this crate's `build.rs`, from facts about the build |
| Seen by dependents | yes, it is part of the API | yes | no (checked: a dependent's `cfg!` of it is `false`, with an `unexpected_cfgs` warning) |
| Unified across the build | yes | yes | no |
| Declared for `unexpected_cfgs` by | Cargo, from `[features]` | Cargo | `cargo::rustc-check-cfg` in `build.rs` |
| In this crate | `from_toml_str`, `UncheckedEmail` | the serde derives and `#[serde(..)]` | `config_lab_has_floor_char_boundary` |

A cfg that `RUSTFLAGS` sets instead of a build script (like `loom` in
`deep-dive/`) is declared in `Cargo.toml` with
`[lints.rust] unexpected_cfgs = { level = "warn", check-cfg = [..] }`.

### The build script

`build.rs` runs `$RUSTC --version` (serde's own build script does the same)
and, on Rust 1.91 or newer, prints
`cargo::rustc-cfg=config_lab_has_floor_char_boundary`. With the cfg,
`truncate_on_char_boundary` uses std's `str::floor_char_boundary` (stable
since 1.91); without it, a hand-written fallback, because the crate supports
Rust 1.85 (`rust-version`: edition 2024, toml 1.x and proptest 1.11 all need
1.85, and CI builds and tests the crate with a 1.85 toolchain, on which the
script leaves the cfg unset). `EmailError` uses it to repeat at most 64
bytes of a rejected address without cutting a character in half, where
`&s[..64]` would panic.

- `cargo::rustc-check-cfg=cfg(config_lab_has_floor_char_boundary)` declares
  the name, even on compilers where the cfg is not set. Without it, every use
  is an `unexpected cfg condition name` warning. A cfg that takes values is
  declared as `cfg(name, values("a", "b"))`, and then a misspelled value
  warns too.
- `cargo::rerun-if-env-changed=CONFIG_LAB_FORCE_FALLBACK` tells Cargo to run
  the script again when that variable changes (`cargo build -v` then says
  "the env variable CONFIG_LAB_FORCE_FALLBACK changed"). `=1` forces the
  fallback on a new compiler, so CI compiles and tests it without an old
  toolchain.
- `cargo::rerun-if-changed=build.rs`: once a script prints any `rerun-if`
  line, Cargo reruns it only for the listed files and variables. With none,
  it reruns it whenever any file in the package changes (verified by
  touching `README.md`). An environment variable is never tracked unless it
  is listed.
- clippy's `incompatible_msrv` lint (warn by default) compares every std API
  call with `rust-version` and does not know about the cfg, so it flags
  `floor_char_boundary`. `#[clippy::msrv = "1.91"]` on the gated function
  tells it what the cfg already guarantees.
- The `cargo::` form of the directives needs Cargo 1.77 or newer. serde,
  which supports Rust 1.56, still prints the older `cargo:` form.

## Compare with

- `35_error_design/err3`: `TryFrom<i32> for Percentage`, a newtype that
  cannot hold an invalid value. `try_from = "String"` makes serde go through
  exactly that kind of constructor, and `EmailError` is the same kind of real
  error type (`Display` + `Error`).
- `47_type_level/builder1`: a hand-written `Default` and a validating
  `build()`. Same idea, one checked way in, for values built in code instead
  of read from a file.
- `47_type_level/typestate1`: the same "an invalid value can't exist" idea
  moved into the type system. `send()` exists only on a
  `RequestBuilder<HasUrl>`, just as `ServiceConfig::admin_email` can only
  hold an `Email` that passed the check.
- `42_coherence/coherence1`: the orphan rule, which is why a type from
  another crate can't get a `Deserialize` impl from you, and why serde offers
  `#[serde(with = "..")]` and `#[serde(remote = "..")]` besides newtypes.
- `deep-dive/Cargo.toml`: `loom` is a dependency that exists only under
  `--cfg loom` (`[target.'cfg(loom)'.dependencies]`), a cfg set through
  `RUSTFLAGS` instead of a feature, declared with `check-cfg` in
  `[lints.rust]`.

## Run it

From the repository root, `cargo test --manifest-path config-lab/Cargo.toml`
runs the tests with the default features. Inside `config-lab/`:

```bash
cargo test                              # default features (serde)
cargo test --no-default-features        # no serde: the TOML tests are skipped
cargo test --all-features               # lenient: 2 typo tests ignored, its demo runs
CONFIG_LAB_FORCE_FALLBACK=1 cargo test  # the pre-1.91 floor_char_boundary path
cargo run --example report              # what this build was compiled with
cargo +nightly test --doc               # nightly also checks the compile_fail code
```

`examples/report.rs` prints the enabled features and which
`floor_char_boundary` the build uses, then parses a config with a typo in
it. Run it with `--no-default-features`, `--features lenient` and
`CONFIG_LAB_FORCE_FALLBACK=1` to see each shape. CI runs `fmt`; clippy with
`-D warnings` and the tests for the default features,
`--no-default-features`, `--all-features` and the forced fallback; rustdoc
with `-D warnings` with and without the default features; and the tests on
Rust 1.85.

Stable rustdoc does not check the error code of a `compile_fail` doctest
(`Email`'s says E0423), so the doctest just above it is its positive
control: the same import and the same string, built through `try_from`
instead of the private constructor. Nightly rustdoc does check the code:
change it to E0603 and `cargo +nightly test --doc` fails with "Some expected
error codes were not found".

The proptest tests set `failure_persistence: None`, so a failing run prints
the shrunk input instead of writing a `proptest-regressions/` file into the
source tree. They also fix the RNG seed, so every run tries the same 256
cases and a failure can be reproduced; set `PROPTEST_RNG_SEED` to another
number to explore other inputs (and `PROPTEST_CASES` to try more of them).

## Try it

Commands below run from `config-lab/`.

1. **Watch the non-additive feature break a guarantee.**
   `cargo test --features lenient -- --ignored` runs the two typo tests that
   `lenient` has to switch off. Both fail with "the input should be
   rejected", and the printed config shows `port: 8080`: the typo was
   dropped. This is the ROADMAP's "remove `deny_unknown_fields`" tinker,
   done by a feature; deleting the two `deny_unknown_fields` `cfg_attr`
   blocks in `src/serde_boundary.rs` by hand fails the same two tests under
   a plain `cargo test`.
2. **Watch feature unification happen.** From the repository root:

   ```bash
   lab="$PWD/config-lab"
   cd "$(mktemp -d)"
   cargo new --lib plugin && cargo new app
   cargo add --manifest-path plugin/Cargo.toml --path "$lab" --features lenient
   cargo add --manifest-path app/Cargo.toml --path "$lab"
   cargo add --manifest-path app/Cargo.toml --path plugin
   cat > app/src/main.rs <<'EOF'
   fn main() {
       let input = "name = \"billing\"\nadmin-email = \"ops@example.com\"\n[server]\nprot = 9000\n";
       let result = config_lab::ServiceConfig::from_toml_str(input);
       println!("{:?}", result.map(|config| config.server.port));
   }
   EOF
   cargo run --manifest-path app/Cargo.toml    # Ok(8080): app never uses plugin
   cargo tree --manifest-path app/Cargo.toml -e features -i config-lab
   cargo remove --manifest-path app/Cargo.toml plugin
   cargo run --manifest-path app/Cargo.toml    # Err(.. unknown field `prot` ..)
   ```

   `cargo tree` shows `config-lab feature "lenient"` coming from `plugin`.
3. **Break the `--no-default-features` build.** Replace `LogLevel`'s two
   `cfg_attr` lines with `#[derive(serde::Deserialize, serde::Serialize)]`
   and `#[serde(rename_all = "lowercase")]`. `cargo test` still passes, and
   `cargo build --no-default-features` fails with E0433 "cannot find module
   or crate `serde` in this scope" and "cannot find attribute `serde` in this
   scope". Only a CI step with that flag notices.
4. **Misspell a cfg.** Add `#[cfg(feature = "serd")]` to an item: the
   warning says "unexpected `cfg` condition value: `serd`" and lists the
   expected values (`default`, `lenient`, `serde`). Then misspell the cfg in
   `USES_STD_FLOOR_CHAR_BOUNDARY` (`config_lab_has_floor_char_boundry`):
   it warns "unexpected `cfg` condition name", and
   `cargo run --example report` now claims the fallback is in use. An
   unknown cfg is simply false, so the lint is the only thing that notices.
5. **Delete the `rustc-check-cfg` line** in `build.rs`: every use of the
   cfg warns "unexpected `cfg` condition name:
   `config_lab_has_floor_char_boundary`", and `-D warnings` turns them into
   errors.
6. **Delete the `rerun-if-env-changed` line**, run
   `cargo run --example report`, then
   `CONFIG_LAB_FORCE_FALLBACK=1 cargo run --example report`. The second run
   still reports std: the script did not rerun, and the cfg is stale.
7. **Delete `#[clippy::msrv = "1.91"]`** and run `cargo clippy`: "current
   MSRV (Minimum Supported Rust Version) is `1.85.0` but this item is stable
   since `1.91.0`".
8. **Tighten a rule and watch proptest shrink.** Set `MAX_LOCAL_LEN` to 8 in
   `src/serde_boundary.rs` and run `cargo test --test email`.
   `generated_addresses_are_accepted` fails with the same "minimal failing
   input" on every run (with the fixed seed, `s = "A.0.aA.aa@A.aA"` when this
   lab was written): a local part of exactly 9 bytes, one over the new limit,
   then one domain label and a two-letter TLD. Run it again with
   `PROPTEST_RNG_SEED=1`, `2`, ...: the characters change, the shape does
   not.
9. **Remove `#[serde(try_from = "String", into = "String")]`** from `Email`.
   The derives now read and write the bare string, `cargo test` fails five
   tests (the invalid-email ones, the normalization one, and the comparison
   with `UncheckedEmail`), and the round-trip property still passes.

## Further reading

- [serde container attributes](https://serde.rs/container-attrs.html) (`try_from`, `into`, `deny_unknown_fields`, `rename_all`, `default`) and [field attributes](https://serde.rs/field-attrs.html)
- [toml](https://docs.rs/toml/1/toml/) (`from_str`, `to_string`, `de::Error::message` and `span`)
- The Cargo Book: [features](https://doc.rust-lang.org/cargo/reference/features.html) (feature unification, "Features should be additive", mutually exclusive features, [resolver version 2](https://doc.rust-lang.org/cargo/reference/features.html#feature-resolver-version-2)) and [build scripts](https://doc.rust-lang.org/cargo/reference/build-scripts.html) (`rerun-if-changed`, `rerun-if-env-changed`, `rustc-cfg`, `rustc-check-cfg`)
- [Checking conditional configurations](https://doc.rust-lang.org/rustc/check-cfg.html) and [its Cargo specifics](https://doc.rust-lang.org/rustc/check-cfg/cargo-specifics.html)
- [Clippy: `incompatible_msrv`](https://rust-lang.github.io/rust-clippy/master/index.html#incompatible_msrv)
- [Parse, don't validate (Alexis King)](https://lexi-lambda.github.io/blog/2019/11/05/parse-don-t-validate/)
- [The proptest book](https://proptest-rs.github.io/proptest/intro.html), including [failure persistence](https://proptest-rs.github.io/proptest/proptest/failure-persistence.html)
- [serde's own build.rs](https://github.com/serde-rs/serde/blob/master/serde/build.rs), a real `$RUSTC --version` probe
