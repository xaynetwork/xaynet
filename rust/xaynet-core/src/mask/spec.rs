use self::AnalyticsFunc::*;
use crate::mask::Model;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, sync::Arc};

// TODO prob remove, don't think this idea will fly.
/// Precursory data
#[derive(Clone, Debug, PartialEq)]
pub enum PreData {
    Model(Arc<Model>),
    Spec(Arc<Analytic>),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
// TODO all i32 for now, later maybe make this generic in M
// also assume for now N is i32, not sure if we need this yet
// TODO using an enum for now, later perhaps traits might work better
pub enum AnalyticsFunc {
    /// argument is length
    Average(usize),
    /// argument is a list (length at least 2) of bounds defining the subintervals
    Histogram(BTreeSet<i32>),
    /// arguments are (lower bound, upper bound)
    Maxima(i32, i32),
    /* Inhabit(Vec<i32>),
     * Sum(usize),
     * Minima {
     *     a: i32,
     *     b: i32,
     * },
     * GlobalSort {
     *     a: i32, // e.g. 0
     *     b: i32,
     * }, // just int ranges for now
     * Quantile {
     *     a: i32,
     *     b: i32,
     * }, // perhaps q not needed since can prob be computed on demand */
}

impl AnalyticsFunc {
    /// some notion of length
    pub fn len(&self) -> usize {
        match *self {
            Average(n) => n,
            Histogram(ref ranges) => ranges.len() - 1,
            Maxima(lb, ub) => (ub - lb).abs() as usize,
        }
    }

    // not sure if it makes sense to have this here but let's see
    pub fn encode(&self, raw: Vec<i32>) -> Vec<i32> {
        match *self {
            Average(n) => enc_avg(raw, n),
            Histogram(ref ranges) => enc_hist(raw, ranges),
            Maxima(lb, ub) => enc_max(raw, lb, ub),
        }
        // TODO match cases
        // Average(n) => take n from raw and copy to encoded
        // Histogram => take 1, ...
        // Maxima => take 1, ...
    }
}

fn enc_avg(mut raw: Vec<i32>, n: usize) -> Vec<i32> {
    raw.resize(n, 0);
    raw
}

fn enc_hist(mut raw: Vec<i32>, ranges: &BTreeSet<i32>) -> Vec<i32> {
    // panics if raw is empty
    let val = raw.remove(0);
    // ranges
    //     .iter()
    //     .map(|v| if *v == val { 1 } else { 0 })
    //     .collect()
    let mut it = ranges.iter().peekable();
    let mut encoded = Vec::new();
    // for lb in it {
    //     if let Some(ub) = it.peek() {
    //         let is_elem = if *lb <= val && val < **ub { 1 } else { 0 };
    //         encoded.push(is_elem)
    //     }
    // }
    while let Some(lb) = it.next() {
        if let Some(ub) = it.peek() {
            let is_elem = if *lb <= val && val < **ub { 1 } else { 0 };
            encoded.push(is_elem)
        }
    }
    encoded
}

fn enc_max(_raw: Vec<i32>, _lb: i32, _ub: i32) -> Vec<i32> {
    // TODO
    vec![]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Analytic {
    name: String,        // thing to measure
    func: AnalyticsFunc, // how it should be aggregated
}

impl Analytic {
    pub fn new(name: String, func: AnalyticsFunc) -> Self {
        Self { name, func }
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn func(&self) -> &AnalyticsFunc {
        &self.func
    }
}

// NOTE let's ignore this for now, concentrate on single-specs
// the interesting thing is what happens when you do spec1.compose(spec2). it's
// not just append underlying vecs. the indices need to be shuffled
pub struct AggregateSpec {
    _funcs: Vec<Analytic>,
}
