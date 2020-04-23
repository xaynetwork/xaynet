use num::{
    rational::BigRational,
    traits::{float::FloatCore, int::PrimInt, Num},
    BigInt,
    BigUint,
};

#[derive(Clone)]
/// A mask configuration.
pub struct MaskConfig<F: FloatCore> {
    order: BigUint,
    exp_shift: usize,
    add_shift: F,
}

impl<F: FloatCore> MaskConfig<F> {
    /// Get a reference to the order of the finite group.
    pub fn order(&'_ self) -> &'_ BigUint {
        &self.order
    }

    /// Get the exponent (to base 10) of the exponential shift.
    pub fn exp_shift(&self) -> usize {
        self.exp_shift
    }

    /// Get the additive shift.
    pub fn add_shift(&self) -> F {
        self.add_shift
    }
}

/// All available mask configurations.
pub enum MaskConfigs {
    PrimeF32M3,
    PrimeF32M3B0,
    PrimeF64M3,
    PrimeF64M3B0,
}

impl MaskConfigs {
    pub fn config<F: FloatCore>(&self) -> MaskConfig<F> {
        match self {
            // safe unwraps: all digits are smaller then the radix
            // safe unwraps: identity type casts
            Self::PrimeF32M3 => {
                let order = BigUint::from_radix_be(
                    &[
                        6, 8, 0, 5, 6, 4, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 8, 1,
                    ],
                    10,
                )
                .unwrap();
                let exp_shift = 45;
                let add_shift = F::from(f32::max_value()).unwrap();
                MaskConfig {
                    order,
                    exp_shift,
                    add_shift,
                }
            }
            Self::PrimeF32M3B0 => {
                let order = BigUint::from_radix_be(&[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 1], 10)
                    .unwrap();
                let exp_shift = 10;
                let add_shift = F::from(1_f32).unwrap();
                MaskConfig {
                    order,
                    exp_shift,
                    add_shift,
                }
            }
            Self::PrimeF64M3 => {
                let order = BigUint::from_radix_be(
                    &[
                        3, 5, 9, 5, 3, 8, 6, 2, 6, 9, 7, 2, 4, 6, 3, 1, 4, 0, 0, 0, 0, 0, 0, 0, 0,
                        0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 9, 3, 8, 7, 4, 0, 1, 9, 6, 6,
                        7, 2, 3, 1, 6, 6, 6, 0, 6, 7, 4, 3, 9, 0, 9, 6, 5, 2, 9, 9, 2, 4, 9, 6, 9,
                        3, 3, 3, 4, 3, 9, 9, 8, 3, 3, 9, 1, 1, 1, 0, 5, 9, 9, 9, 4, 3, 4, 6, 5, 6,
                        4, 4, 0, 0, 7, 1, 3, 3, 0, 9, 9, 7, 2, 1, 5, 5, 1, 8, 2, 8, 2, 6, 3, 8, 1,
                        3, 0, 4, 4, 7, 1, 0, 3, 2, 3, 6, 6, 7, 3, 9, 0, 4, 0, 5, 2, 7, 9, 6, 7, 0,
                        6, 2, 6, 8, 9, 8, 0, 2, 2, 8, 7, 5, 3, 1, 4, 6, 7, 1, 9, 4, 8, 5, 7, 7, 3,
                        0, 1, 5, 3, 3, 4, 1, 4, 3, 9, 6, 4, 6, 9, 7, 1, 9, 0, 4, 8, 5, 0, 4, 3, 0,
                        6, 0, 1, 2, 5, 9, 6, 3, 8, 6, 6, 3, 8, 8, 5, 9, 3, 4, 0, 0, 8, 4, 0, 3, 0,
                        2, 1, 0, 3, 1, 4, 8, 3, 2, 0, 2, 5, 5, 1, 8, 2, 5, 8, 1, 1, 5, 2, 2, 6, 0,
                        5, 1, 8, 9, 4, 0, 3, 4, 4, 7, 7, 8, 4, 3, 5, 8, 4, 6, 5, 0, 1, 4, 9, 4, 2,
                        0, 0, 9, 0, 3, 7, 4, 3, 7, 3, 1, 3, 4, 8, 7, 6, 7, 7, 5, 7, 8, 6, 9, 2, 3,
                        7, 4, 8, 3, 4, 6, 2, 9, 8, 9, 3, 6, 4, 6, 7, 6, 1, 2, 0, 1, 5, 2, 7, 6, 4,
                        0, 1, 6, 2, 4, 8, 8, 7, 6, 5, 4, 0, 5, 0, 2, 9, 9, 4, 4, 3, 3, 9, 2, 5, 1,
                        0, 5, 5, 5, 6, 8, 9, 9, 8, 1, 5, 0, 1, 6, 0, 8, 7, 0, 9, 4, 9, 4, 0, 0, 4,
                        4, 2, 3, 9, 5, 6, 2, 5, 8, 6, 4, 7, 4, 4, 0, 9, 5, 5, 3, 2, 0, 2, 5, 7, 1,
                        2, 3, 7, 8, 7, 9, 3, 5, 4, 9, 3, 4, 7, 6, 1, 0, 4, 1, 3, 2, 7, 7, 6, 7, 2,
                        8, 5, 4, 8, 4, 3, 7, 7, 8, 3, 2, 8, 3, 1, 1, 2, 4, 2, 8, 4, 4, 5, 4, 5, 0,
                        2, 6, 9, 4, 8, 8, 4, 5, 3, 3, 4, 6, 6, 1, 0, 9, 1, 4, 3, 5, 9, 2, 7, 2, 3,
                        6, 8, 8, 6, 2, 7, 8, 6, 0, 5, 1, 7, 2, 8, 9, 6, 5, 4, 5, 5, 7, 4, 6, 3, 9,
                        3, 0, 9, 5, 8, 4, 6, 7, 2, 0, 8, 6, 0, 3, 4, 7, 6, 4, 4, 6, 6, 2, 2, 0, 1,
                        9, 9, 4, 2, 4, 1, 1, 9, 4, 1, 9, 3, 3, 1, 6, 4, 5, 7, 6, 5, 6, 2, 8, 4, 8,
                        4, 7, 0, 5, 0, 1, 3, 5, 2, 9, 9, 4, 0, 3, 1, 4, 9, 6, 9, 7, 2, 6, 1, 1, 9,
                        9, 9, 5, 7, 8, 3, 5, 8, 2, 4, 0, 0, 0, 5, 3, 1, 2, 3, 3, 0, 3, 1, 6, 1, 9,
                        3, 5, 2, 9, 2, 1, 3, 4, 7, 1, 0, 1, 4, 2, 3, 9, 1, 4, 8, 6, 1, 9, 6, 1, 7,
                        3, 8, 0, 3, 5, 6, 5, 9, 3, 0, 1,
                    ],
                    10,
                )
                .unwrap();
                let exp_shift = 324;
                let add_shift = F::from(f64::max_value()).unwrap();
                MaskConfig {
                    order,
                    exp_shift,
                    add_shift,
                }
            }
            Self::PrimeF64M3B0 => {
                let order = BigUint::from_radix_be(
                    &[
                        2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 9,
                    ],
                    10,
                )
                .unwrap();
                let exp_shift = 20;
                let add_shift = F::from(1_f64).unwrap();
                MaskConfig {
                    order,
                    exp_shift,
                    add_shift,
                }
            }
            // Self:: => {
            //     let order = BigUint::from_radix_be(&[], 10).unwrap();
            //     let exp_shift = ;
            //     let add_shift = F::from().unwrap();
            //     MaskConfig {
            //         order,
            //         exp_shift,
            //         add_shift,
            //     }
            // },
        }
    }
}
