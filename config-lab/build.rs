//! Build script: turns a fact about the compiler into a cfg the crate can test.
//!
//! Cargo features are choices made by the crates that depend on this one. The
//! compiler version is not a choice, it is a fact about the build, and only a
//! build script can find it out. serde's own build.rs has this same shape: it
//! runs `$RUSTC --version` and sets `no_*` cfgs for old compilers.
//!
//! The directives, printed to stdout (the `cargo::` form needs Cargo 1.77+):
//!
//! - `rerun-if-changed` / `rerun-if-env-changed` say when to run this script
//!   again. With no `rerun-if` line at all, Cargo reruns it whenever any file
//!   in the package changes; with them, only when a listed file or variable
//!   changes. Cargo never notices a variable read here unless it is listed.
//! - `rustc-check-cfg` declares a custom cfg, so the `unexpected_cfgs` lint
//!   knows the name and can flag a typo in any `#[cfg(..)]` that uses it.
//! - `rustc-cfg` sets it. It applies to this package only: crates that depend
//!   on this one never see it, so unlike a feature it is not public API.

use std::env;
use std::process::Command;

/// Set to `1` to take the fallback path even on a new compiler, so it can be
/// compiled and tested without installing an old toolchain (CI does this).
const FORCE_FALLBACK: &str = "CONFIG_LAB_FORCE_FALLBACK";

/// `str::floor_char_boundary` is stable since Rust 1.91; this crate supports
/// 1.85 (see `rust-version` in Cargo.toml).
const FLOOR_CHAR_BOUNDARY_SINCE: u32 = 91;

fn main() {
    println!("cargo::rerun-if-changed=build.rs");
    println!("cargo::rerun-if-env-changed={FORCE_FALLBACK}");

    // Declared unconditionally: when the cfg is NOT set (old compiler, or the
    // fallback is forced), `#[cfg(not(config_lab_has_floor_char_boundary))]`
    // must still be a known name.
    println!("cargo::rustc-check-cfg=cfg(config_lab_has_floor_char_boundary)");

    let forced = env::var_os(FORCE_FALLBACK).is_some_and(|v| v == "1");
    let new_enough = rustc_minor_version().is_some_and(|m| m >= FLOOR_CHAR_BOUNDARY_SINCE);
    if new_enough && !forced {
        println!("cargo::rustc-cfg=config_lab_has_floor_char_boundary");
    }
}

/// The `N` in `rustc 1.N.x`, or `None` if it can't be found out (the crate then
/// takes the fallback path, which works on every compiler). Cargo sets `RUSTC`
/// to the compiler it will build the crate with.
fn rustc_minor_version() -> Option<u32> {
    let rustc = env::var_os("RUSTC")?;
    let output = Command::new(rustc).arg("--version").output().ok()?;
    let version = String::from_utf8(output.stdout).ok()?;
    let mut pieces = version.split('.');
    if pieces.next() != Some("rustc 1") {
        return None;
    }
    pieces.next()?.parse().ok()
}
