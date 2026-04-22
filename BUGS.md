# indexmap — Injected Bugs

Total mutations: 1

## Bug Index

| # | Variant | Name | Location | Injection | Fix Commit |
|---|---------|------|----------|-----------|------------|
| 1 | `reverse_indices_offbyone_a442475_1` | `reverse_indices_offbyone` | `src/inner.rs` | `patch` | `a442475ab6a0ddefd886aed1938e6486e4aed1f8` |

## Property Mapping

| Variant | Property | Witness(es) |
|---------|----------|-------------|
| `reverse_indices_offbyone_a442475_1` | `ReversePreservesLookup` | `witness_reverse_preserves_lookup_case_three_keys` |

## Framework Coverage

| Property | proptest | quickcheck | crabcheck | hegel |
|----------|---------:|-----------:|----------:|------:|
| `ReversePreservesLookup` | ✓ | ✓ | ✓ | ✓ |

## Bug Details

### 1. reverse_indices_offbyone

- **Variant**: `reverse_indices_offbyone_a442475_1`
- **Location**: `src/inner.rs`
- **Property**: `ReversePreservesLookup`
- **Witness(es)**:
  - `witness_reverse_preserves_lookup_case_three_keys`
- **Source**: [#128](https://github.com/indexmap-rs/indexmap/pull/128) — Fix off-by-one error in reindexing after `reverse`
  > The initial `IndexMap::reverse` implementation reindexed hash-table entries as `len - i` instead of `len - i - 1`, so every stored index was off by one. Lookups after `reverse` then returned the wrong bucket's value or panicked out of bounds.
- **Fix commit**: `a442475ab6a0ddefd886aed1938e6486e4aed1f8` — Fix off-by-one error in reindexing after `reverse`
- **Invariant violated**: After `IndexMap::reverse()`, every previously inserted key must still map to its originally inserted value — only the iteration order flips, the key→value relation is preserved.
- **How the mutation triggers**: The post-reverse reindexing loop in `Core::reverse` rewrites every hash-table entry as `*i = len - *i` instead of the correct `*i = len - *i - 1`. Every stored index is therefore shifted one slot past its true position — the entry originally at index 0 gets index `len` (out of bounds), so subsequent `map.get(k)` either returns the wrong bucket's value or panics when indexing into `self.entries[len]`. Any map with two or more distinct keys exposes the bug; `catch_unwind` inside the property converts the panic into a `PropertyResult::Fail`.
