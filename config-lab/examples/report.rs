//! Prints what this build of config-lab was compiled with, then (with the
//! `serde` feature) parses a config file that has a typo in it. From
//! `config-lab/`:
//!
//! ```text
//! cargo run --example report
//! cargo run --example report --no-default-features
//! cargo run --example report --features lenient
//! CONFIG_LAB_FORCE_FALLBACK=1 cargo run --example report
//! ```

use config_lab::features_and_cfg::{ENABLED_FEATURES, USES_STD_FLOOR_CHAR_BOUNDARY};

fn main() {
    println!("features: {ENABLED_FEATURES:?}");
    println!(
        "floor_char_boundary: {}",
        if USES_STD_FLOOR_CHAR_BOUNDARY {
            "std (build.rs found rustc 1.91 or newer)"
        } else {
            "fallback (older rustc, or CONFIG_LAB_FORCE_FALLBACK=1)"
        }
    );
    // An example is a target of this package, so it sees the same features.
    #[cfg(feature = "serde")]
    parse_a_typo();
    #[cfg(not(feature = "serde"))]
    println!("no serde: ServiceConfig::from_toml_str does not exist in this build");
}

#[cfg(feature = "serde")]
fn parse_a_typo() {
    let input = "name = \"billing\"\nadmin-email = \"ops@example.com\"\n\n[server]\nprot = 9000\n";
    println!("\nparsing:\n{input}");
    match config_lab::ServiceConfig::from_toml_str(input) {
        Ok(config) => println!("accepted, and the port is {}", config.server.port),
        Err(e) => println!("rejected:\n{e}"),
    }
}
