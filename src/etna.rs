//! ETNA framework-neutral property functions for indexmap.
//!
//! Each `property_<name>` is a pure function taking concrete, owned inputs and
//! returning `PropertyResult`. Framework adapters (proptest / quickcheck /
//! crabcheck / hegel) in `src/bin/etna.rs` and deterministic witness tests in
//! `tests/etna_witnesses.rs` both call these functions directly — there is no
//! re-implementation of the invariant inside any adapter.

#![allow(missing_docs)]

use alloc::format;
use alloc::string::{String, ToString};
use alloc::vec::Vec;

use crate::IndexMap;

pub enum PropertyResult {
    Pass,
    Fail(String),
    Discard,
}

// ──────────────────────────────────────────────────────────────────────────
// Property: IndexMap::reverse preserves key → value lookups.
//
// Regression for a442475 — the post-reverse reindexing loop stored
// `len - old_index` instead of `len - old_index - 1`, so every hash-table
// entry pointed one slot past the correct position (the last entry's index
// being out of bounds). After reverse, `map[&key]` therefore returned the
// wrong value (or panicked when the index was `len`).
//
// The property inserts a deduplicated list of `(u16, u16)` pairs, reverses
// the map, and checks that every key still maps to its originally inserted
// value. The iteration order reversal is trivial; the real witness is the
// integrity of the internal index table.
// ──────────────────────────────────────────────────────────────────────────
pub fn property_reverse_preserves_lookup(pairs: Vec<(u16, u16)>) -> PropertyResult {
    // Deduplicate keys while preserving first-seen order. IndexMap::insert
    // on repeated keys would only keep the last value, but we want a clean
    // injective key→value mapping for the lookup check.
    let mut expected: Vec<(u16, u16)> = Vec::new();
    let mut seen: std::collections::HashSet<u16> = std::collections::HashSet::new();
    for (k, v) in pairs {
        if seen.insert(k) {
            expected.push((k, v));
        }
    }
    if expected.is_empty() {
        return PropertyResult::Discard;
    }

    let mut map: IndexMap<u16, u16> = IndexMap::new();
    for (k, v) in &expected {
        map.insert(*k, *v);
    }

    // Wrap the reverse + lookup loop in catch_unwind so the bug's
    // out-of-bounds internal index (which panics on lookup) surfaces as
    // Fail rather than aborting the whole adapter.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> Result<(), String> {
        map.reverse();
        for (k, v) in &expected {
            match map.get(k) {
                Some(got) if got == v => {}
                Some(got) => {
                    return Err(format!(
                        "after reverse: key {} mapped to {} expected {}",
                        k, got, v
                    ));
                }
                None => {
                    return Err(format!("after reverse: key {} missing (expected {})", k, v));
                }
            }
        }
        Ok(())
    }));

    match result {
        Ok(Ok(())) => PropertyResult::Pass,
        Ok(Err(msg)) => PropertyResult::Fail(msg),
        Err(panic) => {
            let msg = if let Some(s) = panic.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = panic.downcast_ref::<&str>() {
                (*s).to_string()
            } else {
                "panic with non-string payload".to_string()
            };
            PropertyResult::Fail(format!("panic during reverse+lookup: {}", msg))
        }
    }
}
