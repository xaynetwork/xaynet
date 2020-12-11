//use num::{bigint::BigInt, rational::Ratio};
use self::AnalyticsFunc::*;
use std::collections::{BTreeSet, HashMap};
use xaynet_core::mask::{Analytic, AnalyticsFunc, FromPrimitives, Model};

const DEFAULT_SCALAR: f64 = 1.0;

#[derive(Clone)]
pub struct Preprocessor {
    /// local store of vector measurements
    measurements: HashMap<String, Vec<i32>>,
    /// scalar weight
    scalar_weight: f64,
}

impl Preprocessor {
    pub fn new(measurements: HashMap<String, Vec<i32>>, scalar_weight: f64) -> Self {
        Self {
            measurements,
            scalar_weight,
        }
    }

    pub fn with_measures(measurements: HashMap<String, Vec<i32>>) -> Self {
        Self {
            measurements,
            scalar_weight: DEFAULT_SCALAR,
        }
    }

    #[allow(dead_code)]
    // TODO might need to return scalar as well... because of the weighted average case
    // feels yucky to return scalar though somehow, for most cases it's just not important
    // maybe just do the masking here? rather not
    // what we need is the 1st class scalar thing. just tuck it under Model to hide it
    // for now yeah look just return a pair
    pub fn interpret(&self, analytic: Analytic) -> Model {
        let z = analytic.name();
        match analytic.func() {
            //AnalyticsFunc::Sum(len) => self.interp_sum(z, len),
            AnalyticsFunc::Average(len) => self.interp_average(z, len),
            AnalyticsFunc::Histogram(ranges) => self.interp_histogram(z, ranges),
            //AnalyticsFunc::GlobalSort { a, b } => self.interp_gsort(z, a, b),
            // etc.
            _ => Model::from_primitives(vec![0; 4].into_iter()).unwrap(), // HACK
        }
    }
    #[allow(dead_code)]
    fn sample_1(&self, name: &String) -> i32 {
        *self.sample(name, 1).first().unwrap()
    }

    #[allow(dead_code)]
    fn sample(&self, name: &String, n: usize) -> Vec<i32> {
        // HACK
        let raw_vals = self.measurements.get(name).unwrap().clone();
        raw_vals.into_iter().take(n).collect()
    }

    #[allow(dead_code)]
    fn interp_sum(&self, name: &String, len: &usize) -> Model {
        let vals = self.sample(name, *len);
        Model::from_primitives(vals.into_iter()).unwrap()
    }

    #[allow(dead_code)]
    fn interp_average(&self, name: &String, len: &usize) -> Model {
        let vals = self.sample(name, *len);
        Model::from_primitives(vals.into_iter()).unwrap()
    }

    #[allow(dead_code)]
    fn interp_histogram(&self, _name: &String, _ranges: &BTreeSet<i32>) -> Model {
        // retrieve the named measurement
        // move along ranges until measurement is >= (assuming ranges sorted)
        // 1 for this location of the vector
        // length of ranges is basically length of vector
        Model::from_primitives(vec![0; 4].into_iter()).unwrap()
    }

    #[allow(dead_code)]
    fn interp_gsort(&self, name: &String, _start: &i32, _end: &i32) -> Model {
        let vals = self.sample(name, 4); // FIXME
        Model::from_primitives(vals.into_iter()).unwrap()
    }

    pub fn measure(&self, spec: &Analytic) -> (Model, f64) {
        let name = spec.name();
        let func = spec.func();
        // will panic if we don't have this in the measurements
        let raws = self.measurements.get(name).unwrap();
        let encoded = func.encode(raws.clone());
        let model = Model::from_primitives(encoded.into_iter()).unwrap();
        let scalar = match func {
            Average(_) => self.scalar_weight,
            _ => DEFAULT_SCALAR,
        };
        (model, scalar)
    }
}
