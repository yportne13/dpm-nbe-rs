//! Link the module with a shadow stack big enough for the recursive checker.
//!
//! The parser, elaborator, evaluator and quotation all recurse over the
//! structure of the program; `dpm-nbe`'s own CLI runs them on a 64 MiB thread
//! stack, and `wasm32-unknown-unknown` has no threads, so the equivalent has to
//! be reserved at link time.
//!
//! This lives in `build.rs` rather than only in `.cargo/config.toml` because
//! cargo discovers that file relative to the *invocation* directory: a
//! `cargo build --manifest-path web/api/.../Cargo.toml` run from the repository
//! root would silently drop it.

fn main() {
    // 64 MiB, matching `dpm_nbe::STACK_SIZE`.
    println!("cargo:rustc-link-arg=-zstack-size=67108864");
    println!("cargo:rerun-if-changed=build.rs");
}
