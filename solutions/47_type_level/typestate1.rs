// Traits & Abstraction · Builders and typestate — part 2: zero-sized state markers, `PhantomData` and a sealed trait (E0425, E0405, E0107, E0308).
//
// Part 1's builder catches a missing host at RUN time: `build()` returns
// `Err(MissingHost)`, and every caller has to handle an error that a correct
// program never produces. Typestate moves that check into the type system.
// The builder gets a type parameter that records how far construction has
// got, `RequestBuilder<NoUrl>` or `RequestBuilder<HasUrl>`, and each method
// exists only in the states where it makes sense:
//
//   - `url()` exists only on `RequestBuilder<NoUrl>`. It consumes the builder
//     and returns a `RequestBuilder<HasUrl>`: a state transition is a method
//     that takes `self` by value and returns a DIFFERENT type. (So a second
//     `url()` call does not compile either.)
//   - `header()` exists in every state and keeps the state: it returns `Self`.
//   - `send()` exists only on `RequestBuilder<HasUrl>`, and it returns a plain
//     `Request`, because there is no error left to report. Calling it without
//     a URL is E0599 "no method named `send` found for struct
//     `RequestBuilder<NoUrl>`", with the note "the method was found for
//     `RequestBuilder<HasUrl>`": a compile error in the CALLER's code.
//
// The markers `NoUrl` and `HasUrl` are unit structs, zero-sized types that
// nothing ever looks at at run time. The builder never stores an `S` either,
// but a type parameter that no field uses is E0392 "type parameter `S` is
// never used". The answer is a `PhantomData<S>` field: zero bytes that tell
// the compiler to treat the struct as if it held an `S` (for auto traits and
// variance; `38_variance/variance1` is about choosing that marker with
// care, but with empty unit structs it makes no difference). So
// typestate costs nothing at run time: no extra bytes (a test compares the
// sizes of the two builders), no branch, no error value. The cost is in the
// API. Signatures get more types, and builders in different states cannot
// share a variable: `if x { b = b.url(..) }` no longer type-checks, because
// the two branches would give `b` two different types.
//
// `S` is bounded by a `State` trait, so a `RequestBuilder<String>` gets none
// of the builder's methods, and `State` is SEALED: its supertrait is a `pub`
// trait inside a private module. Code outside the crate can name `State` but
// can never implement it, because it cannot name the supertrait (rustc's
// error note for such an impl even calls it a "sealed trait"). So the set of
// states is closed: generic code over `S: State` knows it only ever sees these
// two, and the crate may later add items to `State` without breaking anyone,
// since no outside impl exists that would need updating. (A private module at
// the crate root is still visible to the whole crate. The seal protects a
// library from OTHER crates, which is exactly where its users are.)
//
// One more trap: `#[derive(Default)]` on `RequestBuilder<S>` generates
// `impl<S: Default> Default for RequestBuilder<S>`. Give `HasUrl` a `Default`
// too and anyone can conjure a `RequestBuilder<HasUrl>` with no URL in it,
// which is exactly the state the types were supposed to rule out.
//
// Graded tests can only show that correct code compiles. The other half of
// the guarantee, "`send()` without `url()` does not compile", belongs in a
// `compile_fail` doctest in the planned `api-surface-lab`. The last two tests
// get close without one: method lookup tries a type's own (inherent) methods
// before trait methods, so a test-local trait method named `send` is called
// only if the `NoUrl` builder has no `send()` of its own. Try the real thing
// yourself in `main`: `RequestBuilder::default().send();`.
//
// How interviewers probe this: "Make `send()` without `url()` a compile error.
// What does it cost at run time? What is `PhantomData` for? Why seal the
// state trait? Why not just `#[derive(Default)]` the generic builder?"

use std::marker::PhantomData;

// The finished request.
#[derive(Debug, PartialEq, Eq)]
pub struct Request {
    pub url: String,
    pub headers: Vec<(String, String)>,
}

// A `pub` trait inside a private module: other crates cannot name it, so
// they cannot implement it, and as `State`'s supertrait it seals `State`.
mod sealed {
    pub trait Sealed {}
}

// The builder's state. Sealed, so these two are the only states there will
// ever be.
pub trait State: sealed::Sealed {}

// Unit structs are zero-sized: the states exist in the types only.
#[derive(Debug)]
pub struct NoUrl;

#[derive(Debug)]
pub struct HasUrl;

impl sealed::Sealed for NoUrl {}
impl sealed::Sealed for HasUrl {}
impl State for NoUrl {}
impl State for HasUrl {}

// No `SendError` any more: a builder without a URL has no `send()` that could
// fail.

// `S` is only a label, so the struct holds it as a zero-sized `PhantomData<S>`.
// The url is a plain `String`: empty (and unallocated) in the `NoUrl` state,
// and set by the move to `HasUrl`, so `send()` has nothing left to check.
#[must_use]
#[derive(Debug)]
pub struct RequestBuilder<S> {
    url: String,
    headers: Vec<(String, String)>,
    state: PhantomData<S>,
}

// Written by hand, for `NoUrl` only. `#[derive(Default)]` would generate
// `impl<S: Default> Default for RequestBuilder<S>`. That impl is generic, so
// `default()` could not pick the state by itself, and the day someone gave
// `HasUrl` a `Default`, it would hand out `HasUrl` builders with no URL.
impl Default for RequestBuilder<NoUrl> {
    fn default() -> Self {
        RequestBuilder {
            url: String::new(),
            headers: Vec::new(),
            state: PhantomData,
        }
    }
}

// Every state has `header()`. It returns `Self`, so the state stays the same.
impl<S: State> RequestBuilder<S> {
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }
}

impl RequestBuilder<NoUrl> {
    // The transition: consume the `NoUrl` builder and return one of a
    // DIFFERENT type, moving the headers across. There is no `url()` on
    // `RequestBuilder<HasUrl>`, so the URL cannot be set twice either.
    pub fn url(self, url: impl Into<String>) -> RequestBuilder<HasUrl> {
        RequestBuilder {
            url: url.into(),
            headers: self.headers,
            state: PhantomData,
        }
    }
}

impl RequestBuilder<HasUrl> {
    // Only a `HasUrl` builder has `send()`, so it cannot fail and returns a
    // plain `Request`.
    pub fn send(self) -> Request {
        Request {
            url: self.url,
            headers: self.headers,
        }
    }
}

fn main() {
    // You can optionally experiment here.
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::any::TypeId;
    use std::mem::size_of;

    fn header(name: &str, value: &str) -> (String, String) {
        (name.to_string(), value.to_string())
    }

    // Which state is this builder in? Generic over every `S: State`, so the
    // call does not pin the state down by itself.
    fn state_of<S: State + 'static>(_: &RequestBuilder<S>) -> TypeId {
        TypeId::of::<S>()
    }

    // Takes a builder in ANY state, so `header` must exist for every `S`.
    fn traced<S: State>(builder: RequestBuilder<S>) -> RequestBuilder<S> {
        builder.header("X-Trace-Id", "abc123")
    }

    // What the test-local traits below return. Method lookup tries the
    // receiver by value, then by `&`, and at each step inherent methods come
    // before trait methods. So an inherent `send(self)` or `send(&self)` wins
    // over a trait's `send(&self)`, and getting `Absent` back shows that the
    // builder has no such inherent method.
    struct Absent;

    fn is_absent<T: 'static>(_: T) -> bool {
        TypeId::of::<T>() == TypeId::of::<Absent>()
    }

    #[test]
    fn each_step_has_its_state_in_the_type() {
        let builder: RequestBuilder<NoUrl> = RequestBuilder::default();
        let builder: RequestBuilder<NoUrl> = builder.header("Accept", "text/html");
        let builder: RequestBuilder<HasUrl> = builder.url("https://example.com/");
        let builder: RequestBuilder<HasUrl> = builder.header("Accept-Language", "en");
        let request: Request = builder.send();
        assert_eq!(
            request,
            Request {
                url: "https://example.com/".to_string(),
                headers: vec![
                    header("Accept", "text/html"),
                    header("Accept-Language", "en"),
                ],
            }
        );
    }

    #[test]
    fn default_starts_without_a_url() {
        // No annotation anywhere: `default()` has to pick the state by itself.
        // It can if the only `Default` impl is for `RequestBuilder<NoUrl>`.
        // A generic `impl<S: ...> Default for RequestBuilder<S>` leaves `S`
        // open, even when `NoUrl` is the only type that meets the bound.
        let fresh = RequestBuilder::default();
        assert_eq!(state_of(&fresh), TypeId::of::<NoUrl>());
    }

    #[test]
    fn url_moves_the_builder_to_the_has_url_state() {
        let with_url = RequestBuilder::default().url("https://example.com/");
        assert_eq!(state_of(&with_url), TypeId::of::<HasUrl>());
    }

    #[test]
    fn a_request_without_headers_is_fine() {
        let request: Request = RequestBuilder::default()
            .url("https://example.com/health")
            .send();
        assert_eq!(request.url, "https://example.com/health");
        assert!(request.headers.is_empty());
    }

    #[test]
    fn header_works_in_every_state() {
        let no_url: RequestBuilder<NoUrl> = traced(RequestBuilder::default());
        let has_url: RequestBuilder<HasUrl> = traced(no_url.url("https://x.test/"));
        let request: Request = has_url.send();
        assert_eq!(request.url, "https://x.test/");
        // The same header twice is allowed, and both are kept.
        let trace = header("X-Trace-Id", "abc123");
        assert_eq!(request.headers, [trace.clone(), trace]);
    }

    #[test]
    fn optional_headers_keep_the_state() {
        // `header` returns the same type it was given, so rebinding the
        // builder inside an `if` still type-checks.
        for authenticated in [false, true] {
            let mut builder = RequestBuilder::default().url("https://example.com/me");
            if authenticated {
                builder = builder.header("Authorization", "Bearer t0ken");
            }
            let request: Request = builder.send();
            assert_eq!(request.headers.len(), usize::from(authenticated));
        }
    }

    #[test]
    fn the_url_and_headers_are_moved_not_copied() {
        let url = String::from("https://example.com/upload");
        let url_heap = url.as_ptr();
        let token = String::from("Bearer t0ken");
        let token_heap = token.as_ptr();
        // The header is set BEFORE the transition, so it has to survive
        // `url()`, which builds a builder of a different type.
        let request: Request = RequestBuilder::default()
            .header("Authorization", token)
            .url(url)
            .send();
        // Moving a `String` keeps its heap buffer. A copy made while the
        // original is alive (`clone`, `to_string`, ...) cannot share it.
        assert_eq!(request.url.as_ptr(), url_heap);
        assert_eq!(request.headers[0].1.as_ptr(), token_heap);
    }

    #[test]
    fn the_state_costs_no_memory() {
        assert_eq!(size_of::<NoUrl>(), 0);
        assert_eq!(size_of::<HasUrl>(), 0);
        assert_eq!(
            size_of::<RequestBuilder<NoUrl>>(),
            size_of::<RequestBuilder<HasUrl>>()
        );
    }

    #[test]
    fn the_state_trait_is_sealed() {
        // `state_id` only knows `S: State`, so it can call `sealed_id::<S>()`
        // only if every `State` is also a `sealed::Sealed`, which is what
        // declaring `Sealed` as a supertrait of `State` says.
        fn sealed_id<S: sealed::Sealed + 'static>() -> TypeId {
            TypeId::of::<S>()
        }
        fn state_id<S: State + 'static>() -> TypeId {
            sealed_id::<S>()
        }
        assert_eq!(state_id::<NoUrl>(), TypeId::of::<NoUrl>());
        assert_eq!(state_id::<HasUrl>(), TypeId::of::<HasUrl>());
    }

    #[test]
    fn a_builder_without_a_url_has_no_send() {
        trait FallbackSend {
            fn send(&self) -> Absent;
        }
        impl FallbackSend for RequestBuilder<NoUrl> {
            fn send(&self) -> Absent {
                Absent
            }
        }
        let no_url: RequestBuilder<NoUrl> = RequestBuilder::default();
        assert!(
            is_absent(no_url.send()),
            "`send()` must exist on `RequestBuilder<HasUrl>` only"
        );
    }

    #[test]
    fn the_url_cannot_be_set_twice() {
        trait FallbackUrl {
            fn url(&self, url: &str) -> Absent;
        }
        impl FallbackUrl for RequestBuilder<HasUrl> {
            fn url(&self, _: &str) -> Absent {
                Absent
            }
        }
        let has_url: RequestBuilder<HasUrl> = RequestBuilder::default().url("https://example.com/");
        assert!(
            is_absent(has_url.url("https://example.org/")),
            "`url()` must exist on `RequestBuilder<NoUrl>` only"
        );
    }
}
