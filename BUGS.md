# indexmap — Injected Bugs

Total mutations: 1

All variants are patch-based; apply the listed patch to a clean HEAD to reproduce the buggy build. Each `etna/<variant>` branch is a pre-applied snapshot.

## Bug Index

| # | Name | Variant | File | Injection | Fix Commit |
|---|------|---------|------|-----------|------------|
| 1 | `IndexMap::reverse` stores `len - i` instead of `len - i - 1` | `reverse_indices_offbyone_a442475_1` | `patches/reverse_indices_offbyone_a442475_1.patch` | patch | `a442475ab6a0ddefd886aed1938e6486e4aed1f8` |

## Property Mapping

| Variant | Property | Witness(es) |
|---------|----------|-------------|
| `reverse_indices_offbyone_a442475_1` | `property_reverse_preserves_lookup` | `witness_reverse_preserves_lookup_case_three_keys` |

## Framework Coverage

| Property | etna | proptest | quickcheck | crabcheck | hegel |
|----------|:----:|:--------:|:----------:|:---------:|:-----:|
| `property_reverse_preserves_lookup` | ✓ | ✓ | ✓ | ✓ | ✓ |

## Bug Details

### 1. `IndexMap::reverse` stores `len - i` instead of `len - i - 1`

- **Variant**: `reverse_indices_offbyone_a442475_1`
- **Location**: `patches/reverse_indices_offbyone_a442475_1.patch` (applies to `src/inner.rs`)
- **Property**: `property_reverse_preserves_lookup`
- **Witness**: `witness_reverse_preserves_lookup_case_three_keys`
- **Fix commit**: `a442475ab6a0ddefd886aed1938e6486e4aed1f8` — `Fix off-by-one error in reindexing after reverse`
- **Invariant violated**: After `IndexMap::reverse()`, every previously inserted key must still map to its originally inserted value — only the iteration order flips, the key→value relation is preserved.
- **How the mutation triggers**: The post-reverse reindexing loop in `Core::reverse` rewrites every hash-table entry as `*i = len - *i` instead of the correct `*i = len - *i - 1`. Every stored index is therefore shifted one slot past its true position — the entry originally at index 0 gets index `len` (out of bounds), so subsequent `map.get(k)` either returns the wrong bucket's value or panics when indexing into `self.entries[len]`. Any map with two or more distinct keys exposes the bug; `catch_unwind` inside the property converts the panic into a `PropertyResult::Fail`.
