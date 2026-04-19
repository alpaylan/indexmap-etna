# indexmap — ETNA Tasks

Total tasks: 4

ETNA tasks are **mutation/property/witness triplets**. Each row below is one runnable task: the command executes the framework-specific adapter against the buggy variant branch and should report a counterexample.

Run against a variant by first checking out its branch (`git checkout etna/<variant>`) or applying its patch on a clean tree (`git apply patches/<variant>.patch`).

## Task Index

| Task | Variant | Framework | Property | Witness(es) | Command |
|------|---------|-----------|----------|-------------|---------|
| 001 | `reverse_indices_offbyone_a442475_1` | proptest | `property_reverse_preserves_lookup` | `witness_reverse_preserves_lookup_case_three_keys` | `cargo run --release --bin etna -- proptest ReversePreservesLookup` |
| 002 | `reverse_indices_offbyone_a442475_1` | quickcheck | `property_reverse_preserves_lookup` | `witness_reverse_preserves_lookup_case_three_keys` | `cargo run --release --bin etna -- quickcheck ReversePreservesLookup` |
| 003 | `reverse_indices_offbyone_a442475_1` | crabcheck | `property_reverse_preserves_lookup` | `witness_reverse_preserves_lookup_case_three_keys` | `cargo run --release --bin etna -- crabcheck ReversePreservesLookup` |
| 004 | `reverse_indices_offbyone_a442475_1` | hegel | `property_reverse_preserves_lookup` | `witness_reverse_preserves_lookup_case_three_keys` | `cargo run --release --bin etna -- hegel ReversePreservesLookup` |

## Witness catalog

Each witness is a deterministic concrete test in `tests/etna_witnesses.rs`. Base build: passes. Variant-active build: fails.

- `witness_reverse_preserves_lookup_case_three_keys` — `property_reverse_preserves_lookup(vec![(10, 100), (20, 200), (30, 300)])` → `Pass`. Under `reverse_indices_offbyone_a442475_1` the post-reverse reindexing assigns `len - i` instead of `len - i - 1`, so the entry originally at index 0 gets the out-of-bounds index `3`; looking up key 10 panics (or returns the wrong bucket's value), which `catch_unwind` converts into `PropertyResult::Fail`.
