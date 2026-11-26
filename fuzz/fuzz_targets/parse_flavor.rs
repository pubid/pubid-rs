//! Fuzz target: throw arbitrary bytes at every ported flavor's parse.
//!
//! The contract under fuzz: `FlavorImpl::parse` either returns `Ok` or
//! `Err` — it never panics, never aborts, never reads out of bounds,
//! whatever the bytes are. A panic aborts the harness and libFuzzer
//! records the crashing input; the corpus seeds (structure-aware,
//! generated from the vendored fixture rows by `gen_seeds.py`) keep the
//! mutations near the attested input space so coverage reaches the deep
//! grammar rules fast.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pubid_core::flavor_impl;

fuzz_target!(|data: &[u8]| {
    // The input is arbitrary bytes; lossy UTF-8 keeps the flavor
    // parsers in their string-shaped input space while every byte
    // pattern still lands (invalid sequences survive as U+FFFD).
    let input = String::from_utf8_lossy(data);
    for name in pubid_core::implemented_flavors() {
        let Some(impl_) = flavor_impl(name) else {
            continue;
        };
        // The only contract: no panic. Ok and Err are both fine.
        let _ = impl_.parse(&input);
    }
});
