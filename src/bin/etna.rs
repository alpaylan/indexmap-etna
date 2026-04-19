// ETNA workload runner for indexmap.
//
// Usage: cargo run --release --bin etna -- <tool> <property>
//   tool:     etna | proptest | quickcheck | crabcheck | hegel
//   property: ReversePreservesLookup | All
//
// Each invocation emits a single JSON line on stdout and exits 0
// (usage errors exit 2).

use crabcheck::quickcheck as crabcheck_qc;
use crabcheck::quickcheck::Arbitrary as CcArbitrary;
use hegel::{generators as hgen, Hegel, Settings as HegelSettings};
use indexmap::etna::{property_reverse_preserves_lookup, PropertyResult};
use proptest::prelude::*;
use proptest::test_runner::{Config as ProptestConfig, TestCaseError, TestError};
use quickcheck::{Arbitrary as QcArbitrary, Gen, QuickCheck, ResultStatus, TestResult};
use rand::Rng;
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

#[derive(Default, Clone, Copy)]
struct Metrics {
    inputs: u64,
    elapsed_us: u128,
}

impl Metrics {
    fn combine(self, other: Metrics) -> Metrics {
        Metrics {
            inputs: self.inputs + other.inputs,
            elapsed_us: self.elapsed_us + other.elapsed_us,
        }
    }
}

type Outcome = (Result<(), String>, Metrics);

fn to_err(r: PropertyResult) -> Result<(), String> {
    match r {
        PropertyResult::Pass | PropertyResult::Discard => Ok(()),
        PropertyResult::Fail(m) => Err(m),
    }
}

const ALL_PROPERTIES: &[&str] = &["ReversePreservesLookup"];

fn run_all<F: FnMut(&str) -> Outcome>(mut f: F) -> Outcome {
    let mut total = Metrics::default();
    for p in ALL_PROPERTIES {
        let (r, m) = f(p);
        total = total.combine(m);
        if let Err(e) = r {
            return (Err(e), total);
        }
    }
    (Ok(()), total)
}

// ───────────── etna tool: replays the frozen witness input. ─────────────
fn run_etna_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_etna_property);
    }
    let t0 = Instant::now();
    let result = match property {
        "ReversePreservesLookup" => to_err(property_reverse_preserves_lookup(vec![
            (10u16, 100u16),
            (20u16, 200u16),
            (30u16, 300u16),
        ])),
        _ => {
            return (
                Err(format!("Unknown property for etna: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    (result, Metrics { inputs: 1, elapsed_us })
}

// ───────────── shared generator: bounded (u16, u16) pairs. ─────────────
#[derive(Clone)]
struct Pairs16(Vec<(u16, u16)>);

impl fmt::Debug for Pairs16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl fmt::Display for Pairs16 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

// Bounded sizes: collections up to 16 pairs, u16 values drawn from
// 0..=1024 to keep shrinking + counterexample printing compact.
const MAX_PAIRS: usize = 16;
const KEY_RANGE: u16 = 1024;

fn gen_pairs_qc(g: &mut Gen) -> Vec<(u16, u16)> {
    let len = g.random_range(0..=MAX_PAIRS);
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        let k = g.random_range(0..=KEY_RANGE);
        let v = g.random_range(0..=KEY_RANGE);
        out.push((k, v));
    }
    out
}

fn gen_pairs_cc<R: Rng>(rng: &mut R) -> Vec<(u16, u16)> {
    let len = rng.random_range(0..=MAX_PAIRS);
    let mut out = Vec::with_capacity(len);
    for _ in 0..len {
        let k = rng.random_range(0..=KEY_RANGE);
        let v = rng.random_range(0..=KEY_RANGE);
        out.push((k, v));
    }
    out
}

impl QcArbitrary for Pairs16 {
    fn arbitrary(g: &mut Gen) -> Self {
        Pairs16(gen_pairs_qc(g))
    }
}

impl<R: Rng> CcArbitrary<R> for Pairs16 {
    fn generate(rng: &mut R, _n: usize) -> Self {
        Pairs16(gen_pairs_cc(rng))
    }
}

// ───────────── proptest ─────────────
fn pairs_strategy() -> BoxedStrategy<Vec<(u16, u16)>> {
    prop::collection::vec(
        (0u16..=KEY_RANGE, 0u16..=KEY_RANGE),
        0..=MAX_PAIRS,
    )
    .boxed()
}

fn run_proptest_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_proptest_property);
    }
    let counter = Arc::new(AtomicU64::new(0));
    let t0 = Instant::now();
    let mut runner = proptest::test_runner::TestRunner::new(ProptestConfig {
        cases: 1_000_000,
        ..ProptestConfig::default()
    });
    let result: Result<(), String> = match property {
        "ReversePreservesLookup" => {
            let c = counter.clone();
            runner
                .run(&pairs_strategy(), move |v| {
                    c.fetch_add(1, Ordering::Relaxed);
                    let cex = v.clone();
                    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        property_reverse_preserves_lookup(v)
                    }));
                    match res {
                        Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => Ok(()),
                        Ok(PropertyResult::Fail(_)) | Err(_) => {
                            Err(TestCaseError::fail(format!("({:?})", cex)))
                        }
                    }
                })
                .map_err(|e| match e {
                    TestError::Fail(reason, _) => reason.to_string(),
                    other => other.to_string(),
                })
        }
        _ => {
            return (
                Err(format!("Unknown property for proptest: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = counter.load(Ordering::Relaxed);
    (result, Metrics { inputs, elapsed_us })
}

// ───────────── quickcheck (fork with `etna` feature) ─────────────
static QC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn qc_reverse_preserves_lookup(Pairs16(v): Pairs16) -> TestResult {
    QC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        property_reverse_preserves_lookup(v)
    }));
    match res {
        Ok(PropertyResult::Pass) => TestResult::passed(),
        Ok(PropertyResult::Discard) => TestResult::discard(),
        Ok(PropertyResult::Fail(_)) | Err(_) => TestResult::failed(),
    }
}

fn run_quickcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_quickcheck_property);
    }
    QC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let mut qc = QuickCheck::new()
        .tests(1_000_000)
        .max_tests(2_000_000)
        .max_time(Duration::from_secs(86_400));
    let result = match property {
        "ReversePreservesLookup" => {
            qc.quicktest(qc_reverse_preserves_lookup as fn(Pairs16) -> TestResult)
        }
        _ => {
            return (
                Err(format!("Unknown property for quickcheck: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = QC_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match result.status {
        ResultStatus::Finished => Ok(()),
        ResultStatus::Failed { arguments } => Err(format!("({})", arguments.join(" "))),
        ResultStatus::Aborted { err } => Err(format!("quickcheck aborted: {err:?}")),
        ResultStatus::TimedOut => Err("quickcheck timed out".into()),
        ResultStatus::GaveUp => Err(format!(
            "quickcheck gave up: passed={}, discarded={}",
            result.n_tests_passed, result.n_tests_discarded
        )),
    };
    (status, metrics)
}

// ───────────── crabcheck ─────────────
static CC_COUNTER: AtomicU64 = AtomicU64::new(0);

fn cc_reverse_preserves_lookup(Pairs16(v): Pairs16) -> Option<bool> {
    CC_COUNTER.fetch_add(1, Ordering::Relaxed);
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        property_reverse_preserves_lookup(v)
    }));
    match res {
        Ok(PropertyResult::Pass) => Some(true),
        Ok(PropertyResult::Discard) => None,
        Ok(PropertyResult::Fail(_)) | Err(_) => Some(false),
    }
}

fn run_crabcheck_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_crabcheck_property);
    }
    CC_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let cfg = crabcheck_qc::Config { tests: 1_000_000 };
    let result = match property {
        "ReversePreservesLookup" => crabcheck_qc::quickcheck_with_config(
            cfg,
            cc_reverse_preserves_lookup as fn(Pairs16) -> Option<bool>,
        ),
        _ => {
            return (
                Err(format!("Unknown property for crabcheck: {property}")),
                Metrics::default(),
            )
        }
    };
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = CC_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match result.status {
        crabcheck_qc::ResultStatus::Finished => Ok(()),
        crabcheck_qc::ResultStatus::Failed { arguments } => {
            Err(format!("({})", arguments.join(" ")))
        }
        crabcheck_qc::ResultStatus::TimedOut => Err("crabcheck timed out".into()),
        crabcheck_qc::ResultStatus::GaveUp => Err(format!(
            "crabcheck gave up: passed={}, discarded={}",
            result.passed, result.discarded
        )),
        crabcheck_qc::ResultStatus::Aborted { error } => {
            Err(format!("crabcheck aborted: {error}"))
        }
    };
    (status, metrics)
}

// ───────────── hegel (hegeltest 0.3.7) ─────────────
static HG_COUNTER: AtomicU64 = AtomicU64::new(0);

fn hegel_settings() -> HegelSettings {
    use hegel::HealthCheck;
    HegelSettings::new()
        .test_cases(1_000_000)
        .suppress_health_check(HealthCheck::all())
}

fn run_hegel_property(property: &str) -> Outcome {
    if property == "All" {
        return run_all(run_hegel_property);
    }
    HG_COUNTER.store(0, Ordering::Relaxed);
    let t0 = Instant::now();
    let settings = hegel_settings();
    let run_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| match property {
        "ReversePreservesLookup" => {
            Hegel::new(|tc: hegel::TestCase| {
                HG_COUNTER.fetch_add(1, Ordering::Relaxed);
                // hegel's public generator surface doesn't expose `tuples`, so
                // we draw the outer vector length and each pair component as
                // flat integers and assemble the pairs manually.
                let int_gen = || hgen::integers::<u16>().min_value(0).max_value(KEY_RANGE);
                let len_gen = hgen::integers::<usize>().min_value(0).max_value(MAX_PAIRS);
                let len = tc.draw(len_gen);
                let mut v: Vec<(u16, u16)> = Vec::with_capacity(len);
                for _ in 0..len {
                    let k = tc.draw(int_gen());
                    let val = tc.draw(int_gen());
                    v.push((k, val));
                }
                let cex = v.clone();
                let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    property_reverse_preserves_lookup(v)
                }));
                match res {
                    Ok(PropertyResult::Pass) | Ok(PropertyResult::Discard) => {}
                    Ok(PropertyResult::Fail(_)) | Err(_) => panic!("({:?})", cex),
                }
            })
            .settings(settings.clone())
            .run();
        }
        _ => panic!("__unknown_property:{property}"),
    }));
    let elapsed_us = t0.elapsed().as_micros();
    let inputs = HG_COUNTER.load(Ordering::Relaxed);
    let metrics = Metrics { inputs, elapsed_us };
    let status = match run_result {
        Ok(()) => Ok(()),
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "hegel panicked with non-string payload".to_string()
            };
            if let Some(rest) = msg.strip_prefix("__unknown_property:") {
                return (
                    Err(format!("Unknown property for hegel: {rest}")),
                    Metrics::default(),
                );
            }
            Err(msg
                .strip_prefix("Property test failed: ")
                .unwrap_or(&msg)
                .to_string())
        }
    };
    (status, metrics)
}

fn run(tool: &str, property: &str) -> Outcome {
    match tool {
        "etna" => run_etna_property(property),
        "proptest" => run_proptest_property(property),
        "quickcheck" => run_quickcheck_property(property),
        "crabcheck" => run_crabcheck_property(property),
        "hegel" => run_hegel_property(property),
        _ => (Err(format!("Unknown tool: {tool}")), Metrics::default()),
    }
}

fn json_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn emit_json(
    tool: &str,
    property: &str,
    status: &str,
    metrics: Metrics,
    counterexample: Option<&str>,
    error: Option<&str>,
) {
    let cex = counterexample.map_or("null".to_string(), json_str);
    let err = error.map_or("null".to_string(), json_str);
    println!(
        "{{\"status\":{},\"tests\":{},\"discards\":0,\"time\":{},\"counterexample\":{},\"error\":{},\"tool\":{},\"property\":{}}}",
        json_str(status),
        metrics.inputs,
        json_str(&format!("{}us", metrics.elapsed_us)),
        cex,
        err,
        json_str(tool),
        json_str(property),
    );
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <tool> <property>", args[0]);
        eprintln!("Tools: etna | proptest | quickcheck | crabcheck | hegel");
        eprintln!("Properties: ReversePreservesLookup | All");
        std::process::exit(2);
    }
    let (tool, property) = (args[1].as_str(), args[2].as_str());

    // Silence library-under-test panic noise. Frameworks catch their own
    // panics, but the default hook still prints "thread 'main' panicked ..."
    // to stderr.
    let previous_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(|_| {}));
    let caught = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(tool, property)));
    std::panic::set_hook(previous_hook);

    let (result, metrics) = match caught {
        Ok(outcome) => outcome,
        Err(payload) => {
            let msg = if let Some(s) = payload.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = payload.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "panic with non-string payload".to_string()
            };
            emit_json(
                tool,
                property,
                "aborted",
                Metrics::default(),
                None,
                Some(&format!("adapter panic: {msg}")),
            );
            return;
        }
    };

    match result {
        Ok(()) => emit_json(tool, property, "passed", metrics, None, None),
        Err(msg) => emit_json(tool, property, "failed", metrics, Some(&msg), None),
    }
}
