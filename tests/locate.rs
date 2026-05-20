//! Fault-localization integration tests for indexmap.

use std::fmt;

use crabcheck::quickcheck::{Arbitrary, Mutate};
use indexmap::etna::{property_reverse_preserves_lookup, PropertyResult};
use rand::Rng;

const MAX_PAIRS: usize = 16;
const KEY_RANGE: u16 = 1024;

#[derive(Clone)]
struct Pairs16(Vec<(u16, u16)>);
impl fmt::Debug for Pairs16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl<R: Rng> Arbitrary<R> for Pairs16 {
    fn generate(rng: &mut R, _n: usize) -> Self {
        let len = rng.random_range(0..=MAX_PAIRS);
        Pairs16(
            (0..len)
                .map(|_| {
                    (
                        rng.random_range(0..=KEY_RANGE),
                        rng.random_range(0..=KEY_RANGE),
                    )
                })
                .collect(),
        )
    }
}

impl<R: Rng> Mutate<R> for Pairs16 {
    fn mutate(&self, rng: &mut R, _n: usize) -> Self {
        let mut out = self.0.clone();
        match rng.random_range(0u8..3) {
            0 if !out.is_empty() => {
                let i = rng.random_range(0..out.len());
                let (k, v) = out[i];
                if rng.random_bool(0.5) {
                    out[i] = (k.wrapping_add(1), v);
                } else {
                    out[i] = (k, v.wrapping_add(1));
                }
            }
            1 if out.len() < MAX_PAIRS => out.push((
                rng.random_range(0..=KEY_RANGE),
                rng.random_range(0..=KEY_RANGE),
            )),
            _ if !out.is_empty() => {
                out.pop();
            }
            _ => {}
        }
        Pairs16(out)
    }
}

fn to_opt(r: PropertyResult) -> Option<bool> {
    match r {
        PropertyResult::Pass => Some(true),
        PropertyResult::Fail(_) => Some(false),
        PropertyResult::Discard => None,
    }
}

fn prop_reverse_preserves_lookup(Pairs16(v): Pairs16) -> Option<bool> {
    to_opt(property_reverse_preserves_lookup(v))
}

fn emit_locate_json(r: &crabcheck::profiling::LocateResult) {
    use crabcheck::quickcheck::ResultStatus;
    let status = match &r.run.status {
        ResultStatus::Failed { .. } => "Failed",
        ResultStatus::Finished => "Finished",
        ResultStatus::GaveUp => "GaveUp",
        ResultStatus::TimedOut => "TimedOut",
        ResultStatus::Aborted { .. } => "Aborted",
    };
    let top = if let Some(s) = r.top() {
        serde_json::json!({
            "rank": s.rank,
            "file": s.region.file,
            "function": s.region.function,
            "start_line": s.region.start_line,
            "end_line": s.region.end_line,
            "ochiai": s.region.suspiciousness.ochiai,
            "delta": s.region.delta,
            "panic_overlap": s.panic_overlap,
            "confidence": format!("{}", s.confidence),
            "confidence_rule": s.confidence_rule,
        })
    } else {
        serde_json::Value::Null
    };
    let top_5: Vec<_> = r
        .suspects
        .iter()
        .take(5)
        .map(|s| {
            serde_json::json!({
                "rank": s.rank,
                "file": s.region.file,
                "function": s.region.function,
                "start_line": s.region.start_line,
                "end_line": s.region.end_line,
                "confidence": format!("{}", s.confidence),
                "confidence_rule": s.confidence_rule,
                "panic_overlap": s.panic_overlap,
            })
        })
        .collect();
    let diags: Vec<_> = r.diagnostics.iter().map(|d| d.tag()).collect();
    let out = serde_json::json!({
        "status": status,
        "passed": r.run.passed,
        "discarded": r.run.discarded,
        "n_panics": r.n_panics,
        "n_suspects": r.suspects.len(),
        "top": top,
        "top_5": top_5,
        "diagnostics": diags,
    });
    println!("@@LOCATE@@ {}", out);
}

#[test]
fn locate_reverse_preserves_lookup() {
    let report = crabcheck::quickcheck_with_locate!(prop_reverse_preserves_lookup, "indexmap");
    eprintln!("{report}");
    emit_locate_json(&report);
}
