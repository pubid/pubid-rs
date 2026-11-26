//! Fuzz target: the parse -> render -> parse closure, plus the
//! deserialize closure, on arbitrary input.
//!
//! Whatever the bytes: when a flavor parses an input, rendering it and
//! re-parsing must land on the same canonical hash, and
//! `from_hash(to_hash())` must be hash-stable. A panic or an assertion
//! failure aborts the harness and libFuzzer records the input — the
//! differential-in-a-fuzzer that backs the property suite.

#![no_main]

use libfuzzer_sys::fuzz_target;
use pubid_core::conformance::Identifier;
use pubid_core::flavor_impl;

fuzz_target!(|data: &[u8]| {
    let input = String::from_utf8_lossy(data);
    for name in pubid_core::implemented_flavors() {
        let Some(impl_) = flavor_impl(name) else {
            continue;
        };
        let Ok(identifier) = impl_.parse(&input) else {
            continue; // a reject is fine; the closure only binds on parse
        };
        let hash = identifier.to_hash();
        // The deserialize closure: from_hash(to_hash()) keeps the hash.
        let rebuilt = Identifier::from_hash(&hash)
            .unwrap_or_else(|err| panic!("{name}: from_hash failed: {err}"));
        assert!(
            rebuilt.to_hash() == hash,
            "{name}: deserialize drift on {input:?}"
        );
        // The render closure: parse(render(x)) keeps the hash.
        let rendered = identifier.to_human();
        let reparsed = impl_
            .parse(&rendered)
            .unwrap_or_else(|err| panic!("{name}: render {rendered:?} does not re-parse: {err}"));
        assert!(
            reparsed.to_hash() == hash,
            "{name}: render drift on {input:?} -> {rendered:?}"
        );
    }
});
