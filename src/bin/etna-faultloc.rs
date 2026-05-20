use std::fmt;

use crabcheck::profiling::quickcheck;
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

fn main() {
    let args = std::env::args().collect::<Vec<_>>();
    if args.len() < 3 {
        return;
    }
    let result = match (args[1].as_str(), args[2].as_str()) {
        ("crabcheck", "ReversePreservesLookup") => quickcheck(|Pairs16(v)| {
            to_opt(property_reverse_preserves_lookup(v))
        }),
        (a, b) => panic!("Unknown: {a} {b}"),
    };
    println!("Result: {:?}", result);
}
