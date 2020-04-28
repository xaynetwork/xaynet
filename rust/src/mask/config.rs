use std::convert::{TryFrom, TryInto};

use num::{
    bigint::{BigInt, BigUint},
    rational::Ratio,
    traits::{float::FloatCore, identities::Zero, pow::Pow},
};

use crate::PetError;

#[derive(Clone, Debug, PartialEq)]
/// A mask configuration.
pub struct MaskConfig {
    name: MaskConfigs,
    add_shift: Ratio<BigInt>,
    exp_shift: BigInt,
    order: BigUint,
}

impl MaskConfig {
    /// Get the name.
    pub fn name(&self) -> MaskConfigs {
        self.name
    }

    /// Get a reference to the order of the finite group.
    pub fn order(&'_ self) -> &'_ BigUint {
        &self.order
    }

    /// Get the number of bytes needed to represent the largest element of the finite group.
    pub fn element_len(&self) -> usize {
        if self.order.is_zero() {
            1
        } else {
            (self.order() - BigUint::from(1_usize)).to_bytes_le().len()
        }
    }

    /// Get the exponent (to base 10) of the exponential shift.
    pub fn exp_shift(&'_ self) -> &'_ BigInt {
        &self.exp_shift
    }

    /// Get the additive shift.
    pub fn add_shift(&'_ self) -> &'_ Ratio<BigInt> {
        &self.add_shift
    }

    pub fn serialize(&self) -> Vec<u8> {
        [
            (self.name.group_type as u8).to_le_bytes(),
            (self.name.data_type as u8).to_le_bytes(),
            (self.name.bound_type as u8).to_le_bytes(),
            (self.name.model_type as u8).to_le_bytes(),
        ]
        .concat()
    }

    pub fn deserialize(bytes: &[u8]) -> Result<Self, PetError> {
        if bytes.len() == 4 {
            Ok(MaskConfigs {
                group_type: bytes[0].try_into()?,
                data_type: bytes[1].try_into()?,
                bound_type: bytes[2].try_into()?,
                model_type: bytes[3].try_into()?,
            }
            .config())
        } else {
            Err(PetError::InvalidMask)
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum GroupType {
    Prime,
    // pub Power2,
    // pub Integer,
}

impl TryFrom<u8> for GroupType {
    type Error = PetError;

    /// Get the group type. Fails if the encoding is unknown.
    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::Prime),
            // 1 => Ok(Self::Power2),
            // 2 => Ok(Self::Integer),
            _ => Err(Self::Error::InvalidMask),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum DataType {
    F32,
    F64,
}

impl TryFrom<u8> for DataType {
    type Error = PetError;

    /// Get the data type. Fails if the encoding is unknown.
    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::F32),
            1 => Ok(Self::F64),
            _ => Err(Self::Error::InvalidMask),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum BoundType {
    B0 = 0,
    B2 = 2,
    B4 = 4,
    B6 = 6,
    Bmax = 255,
}

impl TryFrom<u8> for BoundType {
    type Error = PetError;

    /// Get the bound type. Fails if the encoding is unknown.
    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::B0),
            2 => Ok(Self::B2),
            4 => Ok(Self::B4),
            6 => Ok(Self::B6),
            255 => Ok(Self::Bmax),
            _ => Err(Self::Error::InvalidMask),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
#[repr(u8)]
pub enum ModelType {
    M3 = 3,
    M6 = 6,
    M9 = 9,
    M12 = 12,
    // Minf = 255,
}

impl TryFrom<u8> for ModelType {
    type Error = PetError;

    /// Get the model type. Fails if the encoding is unknown.
    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            3 => Ok(Self::M3),
            6 => Ok(Self::M6),
            9 => Ok(Self::M9),
            12 => Ok(Self::M12),
            // 255 => Ok(Self::Minf),
            _ => Err(Self::Error::InvalidMask),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
/// A mask configuration name.
pub struct MaskConfigs {
    group_type: GroupType,
    data_type: DataType,
    bound_type: BoundType,
    model_type: ModelType,
}

impl MaskConfigs {
    /// Create a mask configuration name from its parts.
    pub fn from_parts(
        group_type: GroupType,
        data_type: DataType,
        bound_type: BoundType,
        model_type: ModelType,
    ) -> Self {
        MaskConfigs {
            group_type,
            data_type,
            bound_type,
            model_type,
        }
    }

    /// Get the group type.
    pub fn group_type(&self) -> GroupType {
        self.group_type
    }

    /// Get the data type.
    pub fn data_type(&self) -> DataType {
        self.data_type
    }

    /// Get the bound type.
    pub fn bound_type(&self) -> BoundType {
        self.bound_type
    }

    /// Get the model type.
    pub fn model_type(&self) -> ModelType {
        self.model_type
    }

    /// Get the mask configuration corresponding to the name.
    pub fn config(&self) -> MaskConfig {
        use BoundType::{Bmax, B0, B2, B4, B6};
        use DataType::{F32, F64};
        use GroupType::Prime;
        use ModelType::{M12, M3, M6, M9};

        let name = *self;
        // safe unwraps: all numbers are finite
        let add_shift = match self.data_type {
            F32 => match self.bound_type {
                B0 => Ratio::from_float(1_f32).unwrap(),
                B2 => Ratio::from_float(100_f32).unwrap(),
                B4 => Ratio::from_float(10_000_f32).unwrap(),
                B6 => Ratio::from_float(1_000_000_f32).unwrap(),
                Bmax => Ratio::from_float(f32::max_value()).unwrap(),
            },
            F64 => match self.bound_type {
                B0 => Ratio::from_float(1_f64).unwrap(),
                B2 => Ratio::from_float(100_f64).unwrap(),
                B4 => Ratio::from_float(10_000_f64).unwrap(),
                B6 => Ratio::from_float(1_000_000_f64).unwrap(),
                Bmax => Ratio::from_float(f64::max_value()).unwrap(),
            },
        };
        let exp_shift = match self.data_type {
            F32 => match self.bound_type {
                B0 | B2 | B4 | B6 => BigInt::from(10_usize).pow(10_usize),
                Bmax => BigInt::from(10_usize).pow(45_usize),
            },
            F64 => match self.bound_type {
                B0 | B2 | B4 | B6 => BigInt::from(10_usize).pow(20_usize),
                Bmax => BigInt::from(10_usize).pow(324_usize),
            },
        };
        // safe unwraps: all digits are smaller than the radix
        let order = match self.group_type {
            Prime => match self.data_type {
                F32 => match self.bound_type {
                    B0 => match self.model_type {
                        M3 => {
                            BigUint::from_radix_be(&[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 1], 10)
                                .unwrap()
                        }
                        M6 => BigUint::from_radix_be(
                            &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3],
                            10,
                        )
                        .unwrap(),
                        M9 => BigUint::from_radix_be(
                            &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
                            10,
                        )
                        .unwrap(),
                        M12 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3,
                            ],
                            10,
                        )
                        .unwrap(),
                    },
                    B2 => match self.model_type {
                        M3 => BigUint::from_radix_be(
                            &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 1],
                            10,
                        )
                        .unwrap(),
                        M6 => BigUint::from_radix_be(
                            &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 7],
                            10,
                        )
                        .unwrap(),
                        M9 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 6, 9,
                            ],
                            10,
                        )
                        .unwrap(),
                        M12 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 3,
                            ],
                            10,
                        )
                        .unwrap(),
                    },
                    B4 => match self.model_type {
                        M3 => BigUint::from_radix_be(
                            &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3],
                            10,
                        )
                        .unwrap(),
                        M6 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 8, 9,
                            ],
                            10,
                        )
                        .unwrap(),
                        M9 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                6, 9,
                            ],
                            10,
                        )
                        .unwrap(),
                        M12 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 2, 7,
                            ],
                            10,
                        )
                        .unwrap(),
                    },
                    B6 => match self.model_type {
                        M3 => BigUint::from_radix_be(
                            &[2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 1],
                            10,
                        )
                        .unwrap(),
                        M6 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3,
                            ],
                            10,
                        )
                        .unwrap(),
                        M9 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 9,
                            ],
                            10,
                        )
                        .unwrap(),
                        M12 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 1, 3, 1,
                            ],
                            10,
                        )
                        .unwrap(),
                    },
                    Bmax => match self.model_type {
                        M3 => BigUint::from_radix_be(
                            &[
                                6, 8, 0, 5, 6, 4, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 8, 1,
                            ],
                            10,
                        )
                        .unwrap(),
                        M6 => BigUint::from_radix_be(
                            &[
                                6, 8, 0, 5, 6, 4, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3,
                                2, 3,
                            ],
                            10,
                        )
                        .unwrap(),
                        M9 => BigUint::from_radix_be(
                            &[
                                6, 8, 0, 5, 6, 4, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 1, 9, 1,
                            ],
                            10,
                        )
                        .unwrap(),
                        M12 => BigUint::from_radix_be(
                            &[
                                6, 8, 0, 5, 6, 4, 7, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 8, 3,
                            ],
                            10,
                        )
                        .unwrap(),
                    },
                },
                F64 => match self.bound_type {
                    B0 => match self.model_type {
                        M3 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                6, 9,
                            ],
                            10,
                        )
                        .unwrap(),
                        M6 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 2, 7,
                            ],
                            10,
                        )
                        .unwrap(),
                        M9 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 1, 7,
                            ],
                            10,
                        )
                        .unwrap(),
                        M12 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 1, 5, 9,
                            ],
                            10,
                        )
                        .unwrap(),
                    },
                    B2 => match self.model_type {
                        M3 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 9,
                            ],
                            10,
                        )
                        .unwrap(),
                        M6 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 1, 3, 1,
                            ],
                            10,
                        )
                        .unwrap(),
                        M9 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 4, 7,
                            ],
                            10,
                        )
                        .unwrap(),
                        M12 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 3,
                            ],
                            10,
                        )
                        .unwrap(),
                    },
                    B4 => match self.model_type {
                        M3 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 3, 9,
                            ],
                            10,
                        )
                        .unwrap(),
                        M6 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 7, 1,
                            ],
                            10,
                        )
                        .unwrap(),
                        M9 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 7,
                            ],
                            10,
                        )
                        .unwrap(),
                        M12 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 1,
                            ],
                            10,
                        )
                        .unwrap(),
                    },
                    B6 => match self.model_type {
                        M3 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 1, 7,
                            ],
                            10,
                        )
                        .unwrap(),
                        M6 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 1, 5, 9,
                            ],
                            10,
                        )
                        .unwrap(),
                        M9 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 3,
                            ],
                            10,
                        )
                        .unwrap(),
                        M12 => BigUint::from_radix_be(
                            &[
                                2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 3,
                            ],
                            10,
                        )
                        .unwrap(),
                    },
                    Bmax => match self.model_type {
                        M3 => BigUint::from_radix_be(
                            &[
                                3, 5, 9, 5, 3, 8, 6, 2, 6, 9, 7, 2, 4, 6, 3, 1, 4, 0, 0, 0, 0, 0,
                                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 9, 3, 8, 7,
                                4, 0, 1, 9, 6, 6, 7, 2, 3, 1, 6, 6, 6, 0, 6, 7, 4, 3, 9, 0, 9, 6,
                                5, 2, 9, 9, 2, 4, 9, 6, 9, 3, 3, 3, 4, 3, 9, 9, 8, 3, 3, 9, 1, 1,
                                1, 0, 5, 9, 9, 9, 4, 3, 4, 6, 5, 6, 4, 4, 0, 0, 7, 1, 3, 3, 0, 9,
                                9, 7, 2, 1, 5, 5, 1, 8, 2, 8, 2, 6, 3, 8, 1, 3, 0, 4, 4, 7, 1, 0,
                                3, 2, 3, 6, 6, 7, 3, 9, 0, 4, 0, 5, 2, 7, 9, 6, 7, 0, 6, 2, 6, 8,
                                9, 8, 0, 2, 2, 8, 7, 5, 3, 1, 4, 6, 7, 1, 9, 4, 8, 5, 7, 7, 3, 0,
                                1, 5, 3, 3, 4, 1, 4, 3, 9, 6, 4, 6, 9, 7, 1, 9, 0, 4, 8, 5, 0, 4,
                                3, 0, 6, 0, 1, 2, 5, 9, 6, 3, 8, 6, 6, 3, 8, 8, 5, 9, 3, 4, 0, 0,
                                8, 4, 0, 3, 0, 2, 1, 0, 3, 1, 4, 8, 3, 2, 0, 2, 5, 5, 1, 8, 2, 5,
                                8, 1, 1, 5, 2, 2, 6, 0, 5, 1, 8, 9, 4, 0, 3, 4, 4, 7, 7, 8, 4, 3,
                                5, 8, 4, 6, 5, 0, 1, 4, 9, 4, 2, 0, 0, 9, 0, 3, 7, 4, 3, 7, 3, 1,
                                3, 4, 8, 7, 6, 7, 7, 5, 7, 8, 6, 9, 2, 3, 7, 4, 8, 3, 4, 6, 2, 9,
                                8, 9, 3, 6, 4, 6, 7, 6, 1, 2, 0, 1, 5, 2, 7, 6, 4, 0, 1, 6, 2, 4,
                                8, 8, 7, 6, 5, 4, 0, 5, 0, 2, 9, 9, 4, 4, 3, 3, 9, 2, 5, 1, 0, 5,
                                5, 5, 6, 8, 9, 9, 8, 1, 5, 0, 1, 6, 0, 8, 7, 0, 9, 4, 9, 4, 0, 0,
                                4, 4, 2, 3, 9, 5, 6, 2, 5, 8, 6, 4, 7, 4, 4, 0, 9, 5, 5, 3, 2, 0,
                                2, 5, 7, 1, 2, 3, 7, 8, 7, 9, 3, 5, 4, 9, 3, 4, 7, 6, 1, 0, 4, 1,
                                3, 2, 7, 7, 6, 7, 2, 8, 5, 4, 8, 4, 3, 7, 7, 8, 3, 2, 8, 3, 1, 1,
                                2, 4, 2, 8, 4, 4, 5, 4, 5, 0, 2, 6, 9, 4, 8, 8, 4, 5, 3, 3, 4, 6,
                                6, 1, 0, 9, 1, 4, 3, 5, 9, 2, 7, 2, 3, 6, 8, 8, 6, 2, 7, 8, 6, 0,
                                5, 1, 7, 2, 8, 9, 6, 5, 4, 5, 5, 7, 4, 6, 3, 9, 3, 0, 9, 5, 8, 4,
                                6, 7, 2, 0, 8, 6, 0, 3, 4, 7, 6, 4, 4, 6, 6, 2, 2, 0, 1, 9, 9, 4,
                                2, 4, 1, 1, 9, 4, 1, 9, 3, 3, 1, 6, 4, 5, 7, 6, 5, 6, 2, 8, 4, 8,
                                4, 7, 0, 5, 0, 1, 3, 5, 2, 9, 9, 4, 0, 3, 1, 4, 9, 6, 9, 7, 2, 6,
                                1, 1, 9, 9, 9, 5, 7, 8, 3, 5, 8, 2, 4, 0, 0, 0, 5, 3, 1, 2, 3, 3,
                                0, 3, 1, 6, 1, 9, 3, 5, 2, 9, 2, 1, 3, 4, 7, 1, 0, 1, 4, 2, 3, 9,
                                1, 4, 8, 6, 1, 9, 6, 1, 7, 3, 8, 0, 3, 5, 6, 5, 9, 3, 0, 1,
                            ],
                            10,
                        )
                        .unwrap(),
                        M6 => BigUint::from_radix_be(
                            &[
                                3, 5, 9, 5, 3, 8, 6, 2, 6, 9, 7, 2, 4, 6, 3, 1, 3, 9, 9, 9, 9, 9,
                                9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 0, 3, 6, 2,
                                2, 1, 0, 6, 3, 0, 9, 6, 0, 1, 8, 4, 0, 4, 0, 2, 5, 5, 8, 2, 9, 6,
                                2, 6, 1, 3, 6, 0, 0, 5, 5, 8, 4, 3, 4, 6, 0, 1, 6, 3, 7, 1, 4, 9,
                                8, 4, 6, 4, 0, 1, 8, 3, 6, 5, 2, 3, 5, 3, 1, 2, 9, 8, 2, 6, 1, 1,
                                2, 7, 3, 9, 4, 4, 4, 4, 3, 1, 3, 2, 2, 4, 0, 0, 9, 3, 8, 9, 8, 4,
                                1, 5, 2, 6, 0, 0, 5, 7, 5, 4, 2, 1, 5, 9, 1, 2, 1, 2, 7, 3, 9, 5,
                                3, 7, 8, 9, 6, 0, 1, 6, 5, 4, 2, 5, 9, 1, 5, 9, 5, 7, 2, 7, 2, 6,
                                4, 0, 2, 4, 5, 3, 8, 4, 2, 8, 5, 5, 9, 4, 6, 9, 1, 7, 8, 1, 3, 6,
                                6, 1, 1, 6, 8, 0, 8, 8, 1, 7, 1, 0, 1, 5, 0, 8, 1, 8, 0, 8, 9, 7,
                                9, 4, 3, 5, 1, 1, 5, 4, 8, 6, 9, 2, 8, 5, 4, 0, 9, 9, 5, 9, 8, 7,
                                6, 6, 9, 1, 0, 6, 8, 6, 3, 5, 4, 5, 1, 8, 2, 7, 2, 5, 3, 1, 6, 2,
                                8, 4, 4, 0, 5, 8, 7, 9, 1, 3, 4, 3, 4, 8, 7, 2, 8, 6, 8, 5, 2, 6,
                                3, 5, 2, 3, 4, 7, 9, 9, 3, 3, 6, 6, 6, 8, 6, 8, 2, 6, 5, 5, 2, 1,
                                7, 3, 2, 9, 6, 5, 5, 1, 0, 2, 6, 2, 2, 1, 9, 7, 9, 4, 2, 1, 9, 4,
                                2, 1, 2, 8, 5, 7, 6, 5, 8, 8, 3, 4, 0, 4, 3, 4, 6, 5, 7, 1, 3, 8,
                                3, 1, 1, 4, 3, 5, 2, 3, 8, 1, 1, 0, 6, 7, 0, 6, 0, 3, 6, 9, 6, 4,
                                0, 4, 3, 8, 6, 7, 7, 8, 3, 2, 0, 0, 7, 0, 9, 1, 5, 1, 1, 2, 1, 2,
                                7, 8, 8, 3, 9, 8, 4, 7, 0, 3, 9, 1, 2, 8, 5, 3, 2, 0, 7, 2, 0, 7,
                                6, 9, 4, 1, 7, 7, 3, 7, 6, 2, 8, 1, 2, 0, 1, 0, 2, 2, 2, 1, 9, 0,
                                9, 7, 3, 9, 8, 4, 6, 7, 5, 3, 5, 8, 0, 8, 1, 7, 4, 6, 2, 6, 4, 5,
                                6, 0, 2, 8, 5, 4, 4, 9, 6, 1, 0, 3, 8, 6, 6, 3, 2, 7, 4, 7, 4, 1,
                                4, 5, 1, 8, 7, 3, 6, 3, 3, 2, 9, 3, 2, 0, 8, 5, 2, 6, 7, 9, 9, 1,
                                2, 6, 7, 9, 0, 0, 9, 5, 4, 3, 0, 3, 6, 7, 6, 0, 7, 5, 7, 4, 0, 9,
                                7, 2, 0, 5, 7, 4, 1, 9, 1, 3, 3, 8, 8, 3, 2, 8, 4, 1, 1, 0, 4, 1,
                                8, 3, 1, 6, 9, 9, 7, 6, 0, 2, 5, 5, 7, 7, 7, 4, 3, 0, 6, 1, 8, 8,
                                1, 7, 2, 1, 8, 6, 1, 6, 3, 4, 9, 7, 7, 7, 6, 5, 6, 4, 1, 1, 8, 2,
                                9, 9, 6, 1, 9, 4, 5, 7, 3, 4, 4, 8, 6, 2, 6, 7, 6, 3, 7, 2, 0, 9,
                                3, 8, 2, 0, 1, 9, 7, 6, 6, 5, 6, 5, 4, 1, 0, 3, 9, 7, 2, 4, 3, 0,
                                3,
                            ],
                            10,
                        )
                        .unwrap(),
                        M9 => BigUint::from_radix_be(
                            &[
                                3, 5, 9, 5, 3, 8, 6, 2, 6, 9, 7, 2, 4, 6, 3, 1, 3, 9, 9, 9, 9, 9,
                                9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 0, 4, 9, 3,
                                0, 7, 8, 1, 8, 9, 1, 5, 2, 6, 0, 7, 7, 6, 6, 0, 8, 6, 2, 0, 1, 6,
                                9, 6, 6, 4, 3, 7, 7, 6, 6, 4, 7, 8, 9, 3, 4, 8, 2, 0, 8, 8, 5, 7,
                                9, 1, 9, 1, 4, 5, 2, 8, 6, 7, 9, 2, 0, 7, 2, 6, 2, 5, 3, 0, 0, 4,
                                2, 4, 8, 3, 7, 9, 8, 8, 3, 2, 9, 1, 0, 0, 0, 3, 0, 5, 7, 8, 7, 4,
                                9, 5, 8, 3, 1, 0, 6, 9, 4, 4, 8, 4, 5, 1, 7, 1, 3, 9, 8, 4, 1, 1,
                                6, 6, 9, 7, 7, 2, 7, 2, 2, 8, 7, 5, 2, 2, 4, 1, 8, 1, 2, 2, 1, 3,
                                4, 5, 2, 7, 1, 2, 5, 0, 5, 3, 8, 0, 8, 2, 7, 3, 6, 3, 6, 6, 4, 7,
                                1, 8, 1, 9, 0, 3, 3, 8, 3, 7, 1, 7, 4, 1, 8, 1, 6, 9, 7, 8, 2, 2,
                                1, 5, 5, 8, 5, 6, 4, 7, 9, 0, 0, 8, 0, 2, 7, 2, 8, 0, 3, 5, 5, 6,
                                7, 3, 2, 7, 9, 3, 1, 1, 8, 7, 7, 1, 0, 9, 1, 9, 4, 5, 8, 2, 3, 0,
                                9, 5, 7, 0, 3, 6, 5, 1, 1, 5, 0, 7, 1, 5, 0, 2, 8, 8, 1, 3, 7, 8,
                                5, 8, 1, 1, 1, 0, 2, 4, 0, 9, 9, 1, 2, 6, 3, 9, 9, 7, 4, 6, 7, 6,
                                8, 6, 9, 5, 0, 3, 6, 5, 4, 6, 6, 4, 3, 8, 1, 3, 7, 5, 3, 3, 8, 5,
                                0, 6, 2, 3, 8, 5, 7, 6, 2, 6, 5, 2, 3, 8, 0, 1, 5, 0, 3, 4, 6, 6,
                                1, 5, 7, 9, 6, 4, 0, 7, 5, 7, 7, 2, 9, 7, 6, 0, 5, 0, 6, 9, 8, 8,
                                3, 8, 3, 9, 4, 3, 1, 6, 4, 6, 6, 8, 9, 0, 7, 2, 0, 7, 2, 2, 1, 4,
                                6, 8, 7, 5, 8, 4, 0, 9, 9, 3, 5, 6, 2, 7, 3, 9, 5, 9, 0, 2, 5, 5,
                                1, 9, 0, 9, 3, 9, 5, 3, 7, 8, 6, 0, 3, 2, 4, 8, 1, 1, 7, 5, 5, 9,
                                6, 8, 4, 2, 4, 0, 6, 1, 0, 1, 8, 7, 1, 2, 3, 9, 8, 9, 2, 1, 6, 3,
                                5, 0, 5, 5, 2, 7, 1, 3, 7, 5, 1, 9, 5, 6, 9, 0, 4, 6, 7, 4, 7, 9,
                                4, 7, 2, 0, 3, 0, 6, 5, 3, 0, 0, 8, 6, 5, 1, 1, 6, 3, 3, 1, 4, 1,
                                1, 9, 2, 4, 5, 1, 5, 2, 8, 5, 5, 5, 2, 0, 9, 6, 0, 4, 2, 6, 3, 5,
                                8, 7, 4, 4, 7, 4, 9, 6, 0, 7, 3, 3, 4, 4, 5, 2, 4, 1, 4, 5, 1, 7,
                                4, 6, 5, 0, 9, 8, 7, 0, 6, 4, 2, 2, 7, 2, 0, 2, 6, 2, 5, 6, 6, 9,
                                5, 4, 9, 9, 7, 0, 4, 6, 2, 4, 4, 7, 5, 3, 0, 9, 1, 3, 7, 2, 8, 1,
                                6, 4, 4, 3, 5, 8, 1, 8, 3, 3, 7, 3, 1, 6, 0, 0, 6, 8, 5, 2, 3, 6,
                                3, 9, 0, 2, 3, 2, 0, 7, 6, 4, 3, 4, 8, 4, 8, 8, 8, 6, 5, 7, 5, 5,
                                9, 5, 9, 7,
                            ],
                            10,
                        )
                        .unwrap(),
                        M12 => BigUint::from_radix_be(
                            &[
                                3, 5, 9, 5, 3, 8, 6, 2, 6, 9, 7, 2, 4, 6, 3, 1, 3, 9, 9, 9, 9, 9,
                                9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 9, 0, 4, 9, 3,
                                1, 5, 4, 0, 4, 6, 7, 8, 6, 7, 4, 0, 7, 2, 3, 8, 8, 1, 7, 6, 3, 3,
                                4, 4, 7, 1, 1, 4, 2, 0, 3, 7, 5, 9, 6, 6, 4, 6, 2, 0, 7, 8, 7, 4,
                                7, 1, 9, 1, 3, 9, 2, 5, 9, 9, 0, 3, 1, 3, 8, 5, 9, 3, 7, 0, 0, 1,
                                6, 7, 8, 3, 1, 0, 1, 7, 8, 5, 3, 2, 7, 5, 2, 3, 0, 4, 6, 7, 8, 7,
                                2, 4, 7, 0, 9, 0, 9, 7, 8, 9, 3, 1, 0, 4, 2, 2, 3, 6, 1, 2, 8, 2,
                                2, 8, 5, 6, 4, 1, 4, 2, 6, 8, 0, 7, 4, 5, 3, 8, 3, 3, 7, 7, 9, 5,
                                3, 7, 7, 6, 0, 2, 4, 1, 4, 3, 5, 1, 2, 0, 6, 5, 7, 8, 1, 6, 6, 7,
                                9, 7, 8, 5, 2, 5, 7, 4, 8, 3, 0, 0, 2, 4, 1, 6, 5, 9, 4, 2, 5, 1,
                                6, 4, 4, 7, 2, 3, 8, 7, 5, 7, 3, 4, 7, 0, 2, 6, 0, 8, 3, 1, 7, 2,
                                0, 9, 7, 4, 5, 7, 8, 7, 9, 3, 4, 4, 7, 3, 6, 9, 5, 0, 7, 6, 6, 1,
                                7, 3, 9, 4, 9, 0, 2, 1, 8, 8, 0, 6, 7, 9, 0, 0, 0, 1, 7, 6, 5, 1,
                                0, 9, 1, 1, 7, 0, 5, 5, 4, 3, 1, 5, 5, 2, 2, 9, 5, 5, 8, 5, 4, 5,
                                7, 6, 3, 9, 8, 0, 3, 8, 9, 6, 2, 6, 2, 6, 3, 7, 5, 2, 8, 0, 1, 1,
                                8, 9, 7, 2, 4, 2, 3, 1, 6, 4, 2, 6, 0, 7, 9, 4, 0, 0, 3, 9, 2, 7,
                                2, 8, 2, 4, 0, 5, 2, 3, 6, 3, 9, 7, 7, 5, 2, 1, 9, 2, 9, 4, 5, 8,
                                9, 6, 0, 3, 0, 0, 9, 3, 2, 5, 9, 4, 1, 7, 5, 9, 2, 1, 7, 5, 7, 3,
                                3, 4, 0, 6, 2, 6, 0, 6, 3, 7, 1, 6, 8, 3, 8, 6, 7, 1, 3, 1, 5, 1,
                                9, 2, 3, 9, 5, 9, 7, 4, 9, 3, 9, 4, 4, 1, 2, 8, 4, 4, 6, 8, 8, 8,
                                5, 9, 2, 7, 4, 3, 3, 4, 2, 2, 0, 8, 2, 4, 9, 7, 9, 2, 8, 1, 9, 0,
                                2, 5, 4, 1, 9, 0, 9, 3, 5, 7, 1, 7, 3, 3, 7, 4, 5, 2, 7, 4, 1, 8,
                                5, 0, 2, 2, 3, 5, 1, 0, 8, 1, 4, 8, 5, 9, 3, 3, 1, 4, 1, 3, 2, 8,
                                7, 5, 5, 9, 2, 8, 5, 4, 3, 8, 1, 4, 4, 4, 7, 7, 7, 5, 6, 3, 9, 5,
                                5, 8, 3, 8, 7, 8, 7, 6, 1, 3, 1, 3, 2, 9, 5, 1, 3, 0, 5, 6, 7, 3,
                                4, 2, 8, 8, 8, 6, 2, 0, 5, 4, 1, 0, 2, 5, 7, 4, 5, 9, 6, 8, 3, 7,
                                3, 3, 5, 0, 2, 6, 1, 2, 5, 9, 0, 3, 2, 8, 0, 9, 0, 5, 2, 0, 5, 2,
                                4, 7, 5, 3, 0, 1, 4, 9, 6, 4, 1, 6, 1, 2, 8, 3, 7, 2, 3, 0, 0, 0,
                                5, 0, 7, 6, 2, 7, 7, 3, 3, 6, 3, 7, 2, 2, 3, 0, 0, 5, 5, 3, 9, 3,
                                0, 2, 1, 1, 6, 4, 9,
                            ],
                            10,
                        )
                        .unwrap(),
                    },
                },
            },
        };
        MaskConfig {
            name,
            add_shift,
            exp_shift,
            order,
        }
    }
}
