use std::convert::{TryFrom, TryInto};

use num::{
    bigint::{BigInt, BigUint},
    rational::Ratio,
    traits::{identities::Zero, pow::Pow, Num},
};

use crate::PetError;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
/// A mask configuration.
pub struct MaskConfig {
    name: MaskConfigs,
    add_shift: Ratio<BigInt>,
    exp_shift: BigInt,
    order: BigUint,
}

impl MaskConfig {
    derive_struct_fields!(
        name, MaskConfigs;
        add_shift, Ratio<BigInt>;
        exp_shift, BigInt;
        order, BigUint;
    );

    /// Get the number of bytes needed to represent the largest element of the finite group.
    pub fn element_len(&self) -> usize {
        if self.order.is_zero() {
            1
        } else {
            (self.order() - BigUint::from(1_usize)).to_bytes_le().len()
        }
    }

    /// Serialize the mask configuration into bytes.
    pub fn serialize(&self) -> Vec<u8> {
        [
            (self.name.group_type as u8).to_le_bytes(),
            (self.name.data_type as u8).to_le_bytes(),
            (self.name.bound_type as u8).to_le_bytes(),
            (self.name.model_type as u8).to_le_bytes(),
        ]
        .concat()
    }

    /// Deserialize the mask configuration from bytes. Fails if any of its parts is invalid.
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
/// The order of the finite group.
pub enum GroupType {
    /// A finite group of exact integer order.
    Integer,
    /// A finite group of prime order.
    Prime,
    /// A finite group of power-of-two order.
    Power2,
}

impl TryFrom<u8> for GroupType {
    type Error = PetError;

    /// Get the group type. Fails if the encoding is unknown.
    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::Integer),
            1 => Ok(Self::Prime),
            2 => Ok(Self::Power2),
            _ => Err(Self::Error::InvalidMask),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
/// The data type of the numbers to be masked.
pub enum DataType {
    /// Numbers of type f32.
    F32,
    /// Numbers of type f64.
    F64,
    /// Numbers of type i32.
    I32,
    /// Numbers of type i64.
    I64,
}

impl TryFrom<u8> for DataType {
    type Error = PetError;

    /// Get the data type. Fails if the encoding is unknown.
    fn try_from(byte: u8) -> Result<Self, Self::Error> {
        match byte {
            0 => Ok(Self::F32),
            1 => Ok(Self::F64),
            2 => Ok(Self::I32),
            3 => Ok(Self::I64),
            _ => Err(Self::Error::InvalidMask),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
/// The bounds of the numbers to be masked.
pub enum BoundType {
    /// Numbers absolutely bounded by 1.
    B0 = 0,
    /// Numbers absolutely bounded by 100.
    B2 = 2,
    /// Numbers absolutely bounded by 10_000.
    B4 = 4,
    /// Numbers absolutely bounded by 1_000_000.
    B6 = 6,
    /// Numbers absolutely bounded by their data types' maximum absolute value.
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
/// The number of models to be aggregated at most.
pub enum ModelType {
    /// At most 1_000 models to be aggregated.
    M3 = 3,
    /// At most 1_000_000 models to be aggregated.
    M6 = 6,
    /// At most 1_000_000_000 models to be aggregated.
    M9 = 9,
    /// At most 1_000_000_000_000 models to be aggregated.
    M12 = 12,
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
            _ => Err(Self::Error::InvalidMask),
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
/// A mask configuration name. Consists of identifiers for its parts:
/// - the order of the finite group
/// - the data type of the numbers to be masked
/// - the bounds of the numbers to be masked
/// - the number of models to be aggregated at most
pub struct MaskConfigs {
    group_type: GroupType,
    data_type: DataType,
    bound_type: BoundType,
    model_type: ModelType,
}

impl MaskConfigs {
    derive_struct_fields!(
        group_type, GroupType;
        data_type, DataType;
        bound_type, BoundType;
        model_type, ModelType;
    );

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

    /// Get the mask configuration corresponding to the name.
    pub fn config(&self) -> MaskConfig {
        use BoundType::{Bmax, B0, B2, B4, B6};
        use DataType::{F32, F64, I32, I64};
        use GroupType::{Integer, Power2, Prime};
        use ModelType::{M12, M3, M6, M9};

        let name = *self;
        let add_shift = match self.bound_type {
            B0 => Ratio::from_integer(BigInt::from(1)),
            B2 => Ratio::from_integer(BigInt::from(100)),
            B4 => Ratio::from_integer(BigInt::from(10_000)),
            B6 => Ratio::from_integer(BigInt::from(1_000_000)),
            Bmax => match self.data_type {
                // safe unwraps: all numbers are finite
                F32 => Ratio::from_float(f32::MAX).unwrap(),
                F64 => Ratio::from_float(f64::MAX).unwrap(),
                I32 => Ratio::from_integer(-BigInt::from(i32::MIN)),
                I64 => Ratio::from_integer(-BigInt::from(i64::MIN)),
            },
        };
        let exp_shift = match self.data_type {
            F32 => match self.bound_type {
                B0 | B2 | B4 | B6 => BigInt::from(10).pow(10_u8),
                Bmax => BigInt::from(10).pow(45_u8),
            },
            F64 => match self.bound_type {
                B0 | B2 | B4 | B6 => BigInt::from(10).pow(20_u8),
                Bmax => BigInt::from(10).pow(324_u16),
            },
            I32 | I64 => BigInt::from(1),
        };
        let order = match self.group_type {
            Integer => match self.data_type {
                F32 => match self.bound_type {
                    B0 => match self.model_type {
                        // safe unwraps: radix and all strings are valid
                        M3 => BigUint::from_str_radix("20_000_000_000_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("20_000_000_000_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("20_000_000_000_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("20_000_000_000_000_000_000_000", 10).unwrap(),
                    }
                    B2 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_000_000_000_000_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_000_000_000_000_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_000_000_000_000_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_000_000_000_000_000_000_000_000", 10).unwrap(),
                    }
                    B4 => match self.model_type {
                        M3 => BigUint::from_str_radix("200_000_000_000_000_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("200_000_000_000_000_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("200_000_000_000_000_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000", 10).unwrap(),
                    }
                    B6 => match self.model_type {
                        M3 => BigUint::from_str_radix("20_000_000_000_000_000_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("20_000_000_000_000_000_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                    }
                    Bmax => match self.model_type {
                        M3 => BigUint::from_str_radix("680_564_700_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("680_564_700_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("680_564_700_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("680_564_700_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                    }
                }
                F64 => match self.bound_type {
                    B0 => match self.model_type {
                        M3 => BigUint::from_str_radix("200_000_000_000_000_000_000_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                    }
                    B2 => match self.model_type {
                        M3 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                    }
                    B4 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_000_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_000_000_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                    }
                    B6 => match self.model_type {
                        M3 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                    }
                    Bmax => match self.model_type {
                        M3 => BigUint::from_str_radix("359_538_626_972_463_100_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("359_538_626_972_463_100_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("359_538_626_972_463_100_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("359_538_626_972_463_100_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000", 10).unwrap(),
                    }
                }
                I32 => match self.bound_type {
                    B0 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_000_000_000_000", 10).unwrap(),
                    }
                    B2 => match self.model_type {
                        M3 => BigUint::from_str_radix("200_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("200_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("200_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("200_000_000_000_000", 10).unwrap(),
                    }
                    B4 => match self.model_type {
                        M3 => BigUint::from_str_radix("20_000_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("20_000_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("20_000_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("20_000_000_000_000_000", 10).unwrap(),
                    }
                    B6 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_000_000_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_000_000_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_000_000_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_000_000_000_000_000_000", 10).unwrap(),
                    }
                    Bmax => match self.model_type {
                        M3 => BigUint::from_str_radix("4_294_967_295_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("4_294_967_295_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("4_294_967_295_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("4_294_967_295_000_000_000_000", 10).unwrap(),
                    }
                }
                I64 => match self.bound_type {
                    B0 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_000_000_000_000", 10).unwrap(),
                    }
                    B2 => match self.model_type {
                        M3 => BigUint::from_str_radix("200_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("200_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("200_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("200_000_000_000_000", 10).unwrap(),
                    }
                    B4 => match self.model_type {
                        M3 => BigUint::from_str_radix("20_000_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("20_000_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("20_000_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("20_000_000_000_000_000", 10).unwrap(),
                    }
                    B6 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_000_000_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_000_000_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_000_000_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_000_000_000_000_000_000", 10).unwrap(),
                    }
                    Bmax => match self.model_type {
                        M3 => BigUint::from_str_radix("18_446_744_073_709_551_615_000", 10).unwrap(),
                        M6 => BigUint::from_str_radix("18_446_744_073_709_551_615_000_000", 10).unwrap(),
                        M9 => BigUint::from_str_radix("18_446_744_073_709_551_615_000_000_000", 10).unwrap(),
                        M12 => BigUint::from_str_radix("18_446_744_073_709_551_615_000_000_000_000", 10).unwrap(),
                    }
                }
            }
            Prime => match self.data_type {
                F32 => match self.bound_type {
                    B0 => match self.model_type {
                        M3 => BigUint::from_str_radix("20_000_000_000_021", 10).unwrap(),
                        M6 => BigUint::from_str_radix("20_000_000_000_000_003", 10).unwrap(),
                        M9 => BigUint::from_str_radix("20_000_000_000_000_000_011", 10).unwrap(),
                        M12 => BigUint::from_str_radix("20_000_000_000_000_000_000_003", 10).unwrap(),
                    }
                    B2 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_000_000_000_000_021", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_000_000_000_000_000_057", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_000_000_000_000_000_000_069", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_000_000_000_000_000_000_000_003", 10).unwrap(),
                    }
                    B4 => match self.model_type {
                        M3 => BigUint::from_str_radix("200_000_000_000_000_003", 10).unwrap(),
                        M6 => BigUint::from_str_radix("200_000_000_000_000_000_089", 10).unwrap(),
                        M9 => BigUint::from_str_radix("200_000_000_000_000_000_000_069", 10).unwrap(),
                        M12 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_027", 10).unwrap(),
                    }
                    B6 => match self.model_type {
                        M3 => BigUint::from_str_radix("20_000_000_000_000_000_011", 10).unwrap(),
                        M6 => BigUint::from_str_radix("20_000_000_000_000_000_000_003", 10).unwrap(),
                        M9 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_009", 10).unwrap(),
                        M12 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_000_131", 10).unwrap(),
                    }
                    Bmax => match self.model_type {
                        M3 => BigUint::from_str_radix("680_564_700_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_281", 10).unwrap(),
                        M6 => BigUint::from_str_radix("680_564_700_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_323", 10).unwrap(),
                        M9 => BigUint::from_str_radix("680_564_700_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_191", 10).unwrap(),
                        M12 => BigUint::from_str_radix("680_564_700_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_083", 10).unwrap(),
                    }
                }
                F64 => match self.bound_type {
                    B0 => match self.model_type {
                        M3 => BigUint::from_str_radix("200_000_000_000_000_000_000_069", 10).unwrap(),
                        M6 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_027", 10).unwrap(),
                        M9 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_017", 10).unwrap(),
                        M12 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_000_159", 10).unwrap(),
                    }
                    B2 => match self.model_type {
                        M3 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_009", 10).unwrap(),
                        M6 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_000_131", 10).unwrap(),
                        M9 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_000_000_047", 10).unwrap(),
                        M12 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_000_000_000_203", 10).unwrap(),
                    }
                    B4 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_000_000_000_000_000_000_000_000_039", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_000_000_000_000_000_000_000_000_000_071", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_000_000_000_000_000_000_000_000_000_000_017", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_000_000_000_000_000_000_000_000_000_000_000_041", 10).unwrap(),
                    }
                    B6 => match self.model_type {
                        M3 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_017", 10).unwrap(),
                        M6 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_000_159", 10).unwrap(),
                        M9 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_000_000_003", 10).unwrap(),
                        M12 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_000_000_000_023", 10).unwrap(),
                    }
                    Bmax => match self.model_type {
                        M3 => BigUint::from_str_radix("359_538_626_972_463_140_000_000_000_000_000_000_000_593_874_019_667_231_666_067_439_096_529_924_969_333_439_983_391_110_599_943_465_644_007_133_099_721_551_828_263_813_044_710_323_667_390_405_279_670_626_898_022_875_314_671_948_577_301_533_414_396_469_719_048_504_306_012_596_386_638_859_340_084_030_210_314_832_025_518_258_115_226_051_894_034_477_843_584_650_149_420_090_374_373_134_876_775_786_923_748_346_298_936_467_612_015_276_401_624_887_654_050_299_443_392_510_555_689_981_501_608_709_494_004_423_956_258_647_440_955_320_257_123_787_935_493_476_104_132_776_728_548_437_783_283_112_428_445_450_269_488_453_346_610_914_359_272_368_862_786_051_728_965_455_746_393_095_846_720_860_347_644_662_201_994_241_194_193_316_457_656_284_847_050_135_299_403_149_697_261_199_957_835_824_000_531_233_031_619_352_921_347_101_423_914_861_961_738_035_659_301", 10).unwrap(),
                        M6 => BigUint::from_str_radix("359_538_626_972_463_139_999_999_999_999_999_999_999_903_622_106_309_601_840_402_558_296_261_360_055_843_460_163_714_984_640_183_652_353_129_826_112_739_444_431_322_400_938_984_152_600_575_421_591_212_739_537_896_016_542_591_595_727_264_024_538_428_559_469_178_136_611_680_881_710_150_818_089_794_351_154_869_285_409_959_876_691_068_635_451_827_253_162_844_058_791_343_487_286_852_635_234_799_336_668_682_655_217_329_655_102_622_197_942_194_212_857_658_834_043_465_713_831_143_523_811_067_060_369_640_438_677_832_007_091_511_212_788_398_470_391_285_320_720_769_417_737_628_120_102_221_909_739_846_753_580_817_462_645_602_854_496_103_866_327_474_145_187_363_329_320_852_679_912_679_009_543_036_760_757_409_720_574_191_338_832_841_104_183_169_976_025_577_743_061_881_721_861_634_977_765_641_182_996_194_573_448_626_763_720_938_201_976_656_541_039_724_303", 10).unwrap(),
                        M9 => BigUint::from_str_radix("359_538_626_972_463_139_999_999_999_999_999_999_999_904_930_781_891_526_077_660_862_016_966_437_766_478_934_820_885_791_914_528_679_207_262_530_042_483_798_832_910_003_057_874_958_310_694_484_517_139_841_166_977_272_287_522_418_122_134_527_125_053_808_273_636_647_181_903_383_717_418_169_782_215_585_647_900_802_728_035_567_327_931_187_710_919_458_230_957_036_511_507_150_288_137_858_111_024_099_126_399_746_768_695_036_546_643_813_753_385_062_385_762_652_380_150_346_615_796_407_577_297_605_069_883_839_431_646_689_072_072_214_687_584_099_356_273_959_025_519_093_953_786_032_481_175_596_842_406_101_871_239_892_163_505_527_137_519_569_046_747_947_203_065_300_865_116_331_411_924_515_285_552_096_042_635_874_474_960_733_445_241_451_746_509_870_642_272_026_256_695_499_704_624_475_309_137_281_644_358_183_373_160_068_523_639_023_207_643_484_888_657_559_597", 10).unwrap(),
                        M12 => BigUint::from_str_radix("359_538_626_972_463_139_999_999_999_999_999_999_999_904_931_540_467_867_407_238_817_633_447_114_203_759_664_620_787_471_913_925_990_313_859_370_016_783_101_785_327_523_046_787_247_090_978_931_042_236_128_228_564_142_680_745_383_377_953_776_024_143_512_065_781_667_978_525_748_300_241_659_425_164_472_387_573_470_260_831_720_974_578_793_447_369_507_661_739_490_218_806_790_001_765_109_117_055_431_552_295_585_457_639_803_896_262_637_528_011_897_242_316_426_079_400_392_728_240_523_639_775_219_294_589_603_009_325_941_759_217_573_340_626_063_716_838_671_315_192_395_974_939_441_284_468_885_927_433_422_082_497_928_190_254_190_935_717_337_452_741_850_223_510_814_859_331_413_287_559_285_438_144_477_756_395_583_878_761_313_295_130_567_342_888_620_541_025_745_968_373_350_261_259_032_809_052_052_475_301_496_416_128_372_300_050_762_773_363_722_300_553_930_211_649", 10).unwrap(),
                    }
                }
                I32 => match self.bound_type {
                    B0 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_003", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_000_003", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_000_000_011", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_000_000_000_003", 10).unwrap(),
                    }
                    B2 => match self.model_type {
                        M3 => BigUint::from_str_radix("200_003", 10).unwrap(),
                        M6 => BigUint::from_str_radix("200_000_033", 10).unwrap(),
                        M9 => BigUint::from_str_radix("200_000_000_041", 10).unwrap(),
                        M12 => BigUint::from_str_radix("200_000_000_000_027", 10).unwrap(),
                    }
                    B4 => match self.model_type {
                        M3 => BigUint::from_str_radix("20_000_003", 10).unwrap(),
                        M6 => BigUint::from_str_radix("20_000_000_089", 10).unwrap(),
                        M9 => BigUint::from_str_radix("20_000_000_000_021", 10).unwrap(),
                        M12 => BigUint::from_str_radix("20_000_000_000_000_003", 10).unwrap(),
                    }
                    B6 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_000_000_011", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_000_000_000_003", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_000_000_000_000_021", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_000_000_000_000_000_057", 10).unwrap(),
                    }
                    Bmax => match self.model_type {
                        M3 => BigUint::from_str_radix("4_294_967_295_061", 10).unwrap(),
                        M6 => BigUint::from_str_radix("4_294_967_295_000_079", 10).unwrap(),
                        M9 => BigUint::from_str_radix("4_294_967_295_000_000_023", 10).unwrap(),
                        M12 => BigUint::from_str_radix("4_294_967_295_000_000_000_001", 10).unwrap(),
                    }
                }
                I64 => match self.bound_type {
                    B0 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_003", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_000_003", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_000_000_011", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_000_000_000_003", 10).unwrap(),
                    }
                    B2 => match self.model_type {
                        M3 => BigUint::from_str_radix("200_003", 10).unwrap(),
                        M6 => BigUint::from_str_radix("200_000_033", 10).unwrap(),
                        M9 => BigUint::from_str_radix("200_000_000_041", 10).unwrap(),
                        M12 => BigUint::from_str_radix("200_000_000_000_027", 10).unwrap(),
                    }
                    B4 => match self.model_type {
                        M3 => BigUint::from_str_radix("20_000_003", 10).unwrap(),
                        M6 => BigUint::from_str_radix("20_000_000_089", 10).unwrap(),
                        M9 => BigUint::from_str_radix("20_000_000_000_021", 10).unwrap(),
                        M12 => BigUint::from_str_radix("20_000_000_000_000_003", 10).unwrap(),
                    }
                    B6 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_000_000_011", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_000_000_000_003", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_000_000_000_000_021", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_000_000_000_000_000_057", 10).unwrap(),
                    }
                    Bmax => match self.model_type {
                        M3 => BigUint::from_str_radix("18_446_744_073_709_551_615_139", 10).unwrap(),
                        M6 => BigUint::from_str_radix("18_446_744_073_709_551_615_000_053", 10).unwrap(),
                        M9 => BigUint::from_str_radix("18_446_744_073_709_551_615_000_000_133", 10).unwrap(),
                        M12 => BigUint::from_str_radix("18_446_744_073_709_551_615_000_000_000_199", 10).unwrap(),
                    }
                }
            },
            Power2 => match self.data_type {
                F32 => match self.bound_type {
                    B0 => match self.model_type {
                        M3 => BigUint::from_str_radix("35_184_372_088_832", 10).unwrap(),
                        M6 => BigUint::from_str_radix("36_028_797_018_963_968", 10).unwrap(),
                        M9 => BigUint::from_str_radix("36_893_488_147_419_103_232", 10).unwrap(),
                        M12 => BigUint::from_str_radix("37_778_931_862_957_161_709_568", 10).unwrap(),
                    }
                    B2 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_251_799_813_685_248", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_305_843_009_213_693_952", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_361_183_241_434_822_606_848", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_417_851_639_229_258_349_412_352", 10).unwrap(),
                    }
                    B4 => match self.model_type {
                        M3 => BigUint::from_str_radix("288_230_376_151_711_744", 10).unwrap(),
                        M6 => BigUint::from_str_radix("295_147_905_179_352_825_856", 10).unwrap(),
                        M9 => BigUint::from_str_radix("302_231_454_903_657_293_676_544", 10).unwrap(),
                        M12 => BigUint::from_str_radix("309_485_009_821_345_068_724_781_056", 10).unwrap(),
                    }
                    B6 => match self.model_type {
                        M3 => BigUint::from_str_radix("36_893_488_147_419_103_232", 10).unwrap(),
                        M6 => BigUint::from_str_radix("37_778_931_862_957_161_709_568", 10).unwrap(),
                        M9 => BigUint::from_str_radix("38_685_626_227_668_133_590_597_632", 10).unwrap(),
                        M12 => BigUint::from_str_radix("39_614_081_257_132_168_796_771_975_168", 10).unwrap(),
                    }
                    Bmax => match self.model_type {
                        M3 => BigUint::from_str_radix("994_646_472_819_573_284_310_764_496_293_641_680_200_912_301_594_695_434_880_927_953_786_318_994_025_066_751_066_112", 10).unwrap(),
                        M6 => BigUint::from_str_radix("1_018_517_988_167_243_043_134_222_844_204_689_080_525_734_196_832_968_125_318_070_224_677_190_649_881_668_353_091_698_688", 10).unwrap(),
                        M9 => BigUint::from_str_radix("1_042_962_419_883_256_876_169_444_192_465_601_618_458_351_817_556_959_360_325_703_910_069_443_225_478_828_393_565_899_456_512", 10).unwrap(),
                        M12 => BigUint::from_str_radix("1_067_993_517_960_455_041_197_510_853_084_776_057_301_352_261_178_326_384_973_520_803_911_109_862_890_320_275_011_481_043_468_288", 10).unwrap(),
                    }
                }
                F64 => match self.bound_type {
                    B0 => match self.model_type {
                        M3 => BigUint::from_str_radix("302_231_454_903_657_293_676_544", 10).unwrap(),
                        M6 => BigUint::from_str_radix("309_485_009_821_345_068_724_781_056", 10).unwrap(),
                        M9 => BigUint::from_str_radix("316_912_650_057_057_350_374_175_801_344", 10).unwrap(),
                        M12 => BigUint::from_str_radix("324_518_553_658_426_726_783_156_020_576_256", 10).unwrap(),
                    }
                    B2 => match self.model_type {
                        M3 => BigUint::from_str_radix("38_685_626_227_668_133_590_597_632", 10).unwrap(),
                        M6 => BigUint::from_str_radix("39_614_081_257_132_168_796_771_975_168", 10).unwrap(),
                        M9 => BigUint::from_str_radix("20_282_409_603_651_670_423_947_251_286_016", 10).unwrap(),
                        M12 => BigUint::from_str_radix("20_769_187_434_139_310_514_121_985_316_880_384", 10).unwrap(),
                    }
                    B4 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_475_880_078_570_760_549_798_248_448", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_535_301_200_456_458_802_993_406_410_752", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_596_148_429_267_413_814_265_248_164_610_048", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_658_455_991_569_831_745_807_614_120_560_689_152", 10).unwrap(),
                    }
                    B6 => match self.model_type {
                        M3 => BigUint::from_str_radix("316_912_650_057_057_350_374_175_801_344", 10).unwrap(),
                        M6 => BigUint::from_str_radix("324_518_553_658_426_726_783_156_020_576_256", 10).unwrap(),
                        M9 => BigUint::from_str_radix("332_306_998_946_228_968_225_951_765_070_086_144", 10).unwrap(),
                        M12 => BigUint::from_str_radix("340_282_366_920_938_463_463_374_607_431_768_211_456", 10).unwrap(),
                    }
                    Bmax => match self.model_type {
                        M3 => BigUint::from_str_radix("596_143_540_225_991_923_146_302_416_688_458_341_289_203_474_674_553_062_792_993_127_033_853_365_765_018_588_197_722_567_551_977_295_508_215_323_031_793_155_057_153_946_025_631_943_349_443_566_464_703_583_960_364_782_216_884_718_655_637_955_371_883_889_285_523_680_681_542_682_622_992_485_998_454_422_254_346_205_188_269_982_058_330_848_165_814_218_528_432_304_958_458_516_472_675_321_199_923_576_436_128_746_194_040_030_388_187_813_654_706_961_312_852_788_047_760_914_640_519_973_439_182_188_222_756_017_424_664_821_230_981_616_162_111_762_973_371_192_278_908_910_941_031_147_045_555_738_506_834_254_728_517_124_812_756_790_583_181_174_762_115_337_827_697_771_072_593_076_558_961_853_936_203_969_690_859_453_400_618_497_370_766_001_868_317_217_344_149_071_638_768_630_396_860_838_478_405_181_466_899_321_747_678_290_733_613_480_879_657_473_540_096", 10).unwrap(),
                        M6 => BigUint::from_str_radix("610_450_985_191_415_729_301_813_674_688_981_341_480_144_358_066_742_336_300_024_962_082_665_846_543_379_034_314_467_909_173_224_750_600_412_490_784_556_190_778_525_640_730_247_109_989_830_212_059_856_469_975_413_536_990_089_951_903_373_266_300_809_102_628_376_249_017_899_707_005_944_305_662_417_328_388_450_514_112_788_461_627_730_788_521_793_759_773_114_680_277_461_520_868_019_528_908_721_742_270_595_836_102_696_991_117_504_321_182_419_928_384_361_254_960_907_176_591_892_452_801_722_560_740_102_161_842_856_776_940_525_174_950_002_445_284_732_100_893_602_724_803_615_894_574_649_076_230_998_276_842_001_535_808_262_953_557_177_522_956_406_105_935_562_517_578_335_310_396_376_938_430_672_864_963_440_080_282_233_341_307_664_385_913_156_830_560_408_649_358_099_077_526_385_498_601_886_905_822_104_905_469_622_569_711_220_204_420_769_252_905_058_304", 10).unwrap(),
                        M9 => BigUint::from_str_radix("625_101_808_836_009_706_805_057_202_881_516_893_675_667_822_660_344_152_371_225_561_172_649_826_860_420_131_138_015_138_993_382_144_614_822_390_563_385_539_357_210_256_107_773_040_629_586_137_149_293_025_254_823_461_877_852_110_749_054_224_692_028_521_091_457_278_994_329_299_974_086_968_998_315_344_269_773_326_451_495_384_706_796_327_446_316_810_007_669_432_604_120_597_368_851_997_602_531_064_085_090_136_169_161_718_904_324_424_890_798_006_665_585_925_079_968_948_830_097_871_668_963_902_197_864_613_727_085_339_587_097_779_148_802_503_971_565_671_315_049_190_198_902_676_044_440_654_060_542_235_486_209_572_667_661_264_442_549_783_507_359_852_478_016_018_000_215_357_845_889_984_953_009_013_722_562_642_209_006_941_499_048_331_175_072_594_493_858_456_942_693_455_387_018_750_568_332_191_561_835_423_200_893_511_384_289_489_326_867_714_974_779_703_296", 10).unwrap(),
                        M12 => BigUint::from_str_radix("640_104_252_248_073_939_768_378_575_750_673_299_123_883_850_404_192_412_028_134_974_640_793_422_705_070_214_285_327_502_329_223_316_085_578_127_936_906_792_301_783_302_254_359_593_604_696_204_440_876_057_860_939_224_962_920_561_407_031_526_084_637_205_597_652_253_690_193_203_173_465_056_254_274_912_532_247_886_286_331_273_939_759_439_305_028_413_447_853_498_986_619_491_705_704_445_544_991_809_623_132_299_437_221_600_158_028_211_088_177_158_825_559_987_281_888_203_602_020_220_589_019_035_850_613_364_456_535_387_737_188_125_848_373_764_066_883_247_426_610_370_763_676_340_269_507_229_757_995_249_137_878_602_411_685_134_789_170_978_311_536_488_937_488_402_432_220_526_434_191_344_591_881_230_051_904_145_622_023_108_095_025_491_123_274_336_761_711_059_909_318_098_316_307_200_581_972_164_159_319_473_357_714_955_657_512_437_070_712_540_134_174_416_175_104", 10).unwrap(),
                    }
                }
                I32 => match self.bound_type {
                    B0 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_048", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_097_152", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_147_483_648", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_199_023_255_552", 10).unwrap(),
                    }
                    B2 => match self.model_type {
                        M3 => BigUint::from_str_radix("262_144", 10).unwrap(),
                        M6 => BigUint::from_str_radix("268_435_456", 10).unwrap(),
                        M9 => BigUint::from_str_radix("274_877_906_944", 10).unwrap(),
                        M12 => BigUint::from_str_radix("281_474_976_710_656", 10).unwrap(),
                    }
                    B4 => match self.model_type {
                        M3 => BigUint::from_str_radix("33_554_432", 10).unwrap(),
                        M6 => BigUint::from_str_radix("34_359_738_368", 10).unwrap(),
                        M9 => BigUint::from_str_radix("35_184_372_088_832", 10).unwrap(),
                        M12 => BigUint::from_str_radix("36_028_797_018_963_968", 10).unwrap(),
                    }
                    B6 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_147_483_648", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_199_023_255_552", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_251_799_813_685_248", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_305_843_009_213_693_952", 10).unwrap(),
                    }
                    Bmax => match self.model_type {
                        M3 => BigUint::from_str_radix("4_398_046_511_104", 10).unwrap(),
                        M6 => BigUint::from_str_radix("4_503_599_627_370_496", 10).unwrap(),
                        M9 => BigUint::from_str_radix("4_611_686_018_427_387_904", 10).unwrap(),
                        M12 => BigUint::from_str_radix("4_722_366_482_869_645_213_696", 10).unwrap(),
                    }
                }
                I64 => match self.bound_type {
                    B0 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_048", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_097_152", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_147_483_648", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_199_023_255_552", 10).unwrap(),
                    }
                    B2 => match self.model_type {
                        M3 => BigUint::from_str_radix("262_144", 10).unwrap(),
                        M6 => BigUint::from_str_radix("268_435_456", 10).unwrap(),
                        M9 => BigUint::from_str_radix("274_877_906_944", 10).unwrap(),
                        M12 => BigUint::from_str_radix("281_474_976_710_656", 10).unwrap(),
                    }
                    B4 => match self.model_type {
                        M3 => BigUint::from_str_radix("33_554_432", 10).unwrap(),
                        M6 => BigUint::from_str_radix("34_359_738_368", 10).unwrap(),
                        M9 => BigUint::from_str_radix("35_184_372_088_832", 10).unwrap(),
                        M12 => BigUint::from_str_radix("36_028_797_018_963_968", 10).unwrap(),
                    }
                    B6 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_147_483_648", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_199_023_255_552", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_251_799_813_685_248", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_305_843_009_213_693_952", 10).unwrap(),
                    }
                    Bmax => match self.model_type {
                        M3 => BigUint::from_str_radix("18_889_465_931_478_580_854_784", 10).unwrap(),
                        M6 => BigUint::from_str_radix("19_342_813_113_834_066_795_298_816", 10).unwrap(),
                        M9 => BigUint::from_str_radix("19_807_040_628_566_084_398_385_987_584", 10).unwrap(),
                        M12 => BigUint::from_str_radix("20_282_409_603_651_670_423_947_251_286_016", 10).unwrap(),
                    }
                }
            }
        };
        MaskConfig {
            name,
            add_shift,
            exp_shift,
            order,
        }
    }
}
