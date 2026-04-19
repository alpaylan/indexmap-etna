//! Deterministic witness tests for indexmap ETNA variants.
//!
//! Each `witness_<name>_case_<tag>` passes on the base HEAD and fails under the
//! corresponding `etna/<variant>` branch. Witnesses call `property_<name>`
//! directly with frozen inputs — no proptest / quickcheck / RNG / clock.

use indexmap::etna::{property_reverse_preserves_lookup, PropertyResult};

fn expect_pass(r: PropertyResult, what: &str) {
    match r {
        PropertyResult::Pass => {}
        PropertyResult::Fail(m) => panic!("{what}: property failed: {m}"),
        PropertyResult::Discard => panic!("{what}: unexpected discard"),
    }
}

// Variant: reverse_indices_offbyone_a442475_1
// Three distinct keys — after reverse, the bug maps key 10 (stored index 0)
// to len (=3) which is out of bounds, causing either a wrong value or a
// panic on map.get. The smallest witness is a map with ≥2 entries so the
// off-by-one changes a live entry's hash-table slot.
#[test]
fn witness_reverse_preserves_lookup_case_three_keys() {
    expect_pass(
        property_reverse_preserves_lookup(vec![(10u16, 100u16), (20u16, 200u16), (30u16, 300u16)]),
        "reverse_preserves_lookup / three_keys",
    );
}
