use std::env;

// This build script exists as a workaround for the following issue:
// https://github.com/rust-lang/rust/issues/34283
//
// This issue is present under Windows in debug builds.
//
// The CLI has a long `match` statement in the body of a function. LLVM makes it
// so that the stack space required by the `match` statement is proportional to
// the sum of the stack space requirements for each branch, rather than to the
// maximum for all of the branches (which is what happens on higher optimization
// levels and on different targets).
//
// As a result, Windows debug builds have too high a stack utilization, and will
// result in a stack overflow when run. By expanding the available stack space
// at link time, we prevent this from happening.
//
// We want to make sure this variation is only applied to the affected target(s)
// as there are no advantages to a larger stack space other than preventing this
// issue in this specific configuration.

fn main() {
    let os = env::var("CARGO_CFG_TARGET_OS").unwrap();
    let profile = env::var("PROFILE").unwrap();

    if os == "windows" && profile == "debug" {
        println!("cargo:rustc-link-arg=/STACK:0x800000");
    }
}

