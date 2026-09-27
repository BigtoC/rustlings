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
// `compile_fail` doctest, as in `deep-dive/src/api_surface.rs`. The last two
// tests get close without one: method lookup tries a type's own (inherent)
// methods before trait methods, so a test-local trait method named `send` is
// called only if the `NoUrl` builder has no `send()` of its own. Try the real
// thing yourself in `main`: `RequestBuilder::default().send();`.
//
// How interviewers probe this: "Make `send()` without `url()` a compile error.
// What does it cost at run time? What is `PhantomData` for? Why seal the
// state trait? Why not just `#[derive(Default)]` the generic builder?"

// The finished request.
#[derive(Debug, PartialEq, Eq)]
pub struct Request {
    pub url: String,
    pub headers: Vec<(String, String)>,
}

// TODO: the tests name the states `NoUrl` and `HasUrl` and the trait `State`,
// none of which exist yet: E0425 "cannot find type `NoUrl` in this scope" (and
// `HasUrl`), E0405 "cannot find trait `State` in this scope" and E0433
// "cannot find module or crate `sealed`". An unresolved trait in a bound
// stops rustc before it type-checks the function bodies, so the E0308 and
// most of the E0107s described below only show up once all four names exist.
// Declare the two states as zero-sized marker types (a test checks their size)
// and a `State` trait implemented for exactly those two. Seal it: its
// supertrait must be a trait named `Sealed` in a private module named `sealed`
// (a test checks that every `State` is a `sealed::Sealed`). Until you declare
// the markers and both traits, this exercise will not compile.

// The run-time-checked version. Everything below becomes typestate.
#[derive(Debug, PartialEq, Eq)]
pub enum SendError {
    MissingUrl,
}

// TODO: once the markers exist, every `RequestBuilder<NoUrl>` and
// `RequestBuilder<HasUrl>` in the tests is E0107 "struct takes 0 generic
// arguments but 1 generic argument was supplied" (and `state_of` gets E0283
// "type annotations needed", for the same reason). Give the builder a type
// parameter for its state, and keep the state free: both builders must have
// the same size (a test checks). The only way to get a builder from nothing is
// `RequestBuilder::default()`, and it must start in `NoUrl` WITHOUT a type
// annotation (a test relies on that), so `Default` may exist for the `NoUrl`
// builder only, not for a generic `RequestBuilder<S>`. Keep `#[must_use]`.
// Until you give the struct its state parameter, this exercise will not
// compile.
#[must_use]
#[derive(Debug, Default)]
pub struct RequestBuilder {
    url: Option<String>,
    headers: Vec<(String, String)>,
}

// TODO: the tests take `send()`'s result as a `Request`, so rustc reports
// E0308 "mismatched types" (expected `Request`, found `Result<Request,
// SendError>`). Split the methods by state: `url()` only on a `NoUrl`
// builder, turning it into a `HasUrl` builder that keeps the headers set so
// far; `header()` on every state, keeping the state; `send()` only on a
// `HasUrl` builder, returning a plain `Request` (it cannot fail any more, so
// nothing is left to unwrap). The url and the header strings must be moved
// along, never copied (the tests compare heap pointers), and the last two
// tests check that a `NoUrl` builder has no `send()` and a `HasUrl` builder
// has no `url()`. Until you split the methods by state, this exercise will not
// compile.
impl RequestBuilder {
    pub fn url(mut self, url: impl Into<String>) -> Self {
        self.url = Some(url.into());
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.push((name.into(), value.into()));
        self
    }

    pub fn send(self) -> Result<Request, SendError> {
        let url = self.url.ok_or(SendError::MissingUrl)?;
        Ok(Request {
            url,
            headers: self.headers,
        })
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
