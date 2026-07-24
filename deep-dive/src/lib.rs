//! # Deep-dive labs (advanced unsafe exercises)
//!
//! These labs are the part of the advanced course that **requires `unsafe`**,
//! so they live outside rustlings' `unsafe_code = "forbid"` constraint, as a
//! "read + tinker" lab rather than a rustlings-style challenge. Every module
//! ships with unit tests. Run:
//!
//! ```text
//! cargo test --manifest-path deep-dive/Cargo.toml
//! ```
//!
//! | Module | Course module | What it reveals |
//! | --- | --- | --- |
//! | [`unsafe_list`] | M2 data structures | A doubly linked list built on `NonNull` raw pointers, just like the standard library's `LinkedList` |
//! | [`raw_waker`] | M3 async | The real memory representation of a `Waker`: `RawWaker` + a four-function-pointer vtable |
//! | [`self_referential`] | M3 async | Why a self-referential struct cannot be moved, and what `Pin` is really protecting |

pub mod raw_waker;
pub mod self_referential;
pub mod unsafe_list;
