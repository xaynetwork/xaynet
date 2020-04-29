use std::convert::{TryFrom, TryInto};

use num::{
    bigint::{BigInt, BigUint},
    rational::Ratio,
    traits::{identities::Zero, pow::Pow, Num},
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
    I32,
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
        use DataType::{F32, F64, I32, I64};
        use GroupType::Prime;
        use ModelType::{M12, M3, M6, M9};

        let name = *self;
        // safe unwraps: all numbers are finite
        let add_shift = match self.bound_type {
            B0 => Ratio::from_integer(BigInt::from(1)),
            B2 => Ratio::from_integer(BigInt::from(100)),
            B4 => Ratio::from_integer(BigInt::from(10_000)),
            B6 => Ratio::from_integer(BigInt::from(1_000_000)),
            Bmax => match self.data_type {
                F32 => Ratio::from_float(f32::MAX).unwrap(),
                F64 => Ratio::from_float(f64::MAX).unwrap(),
                I32 => Ratio::from_integer(BigInt::from(i32::MAX)),
                I64 => Ratio::from_integer(BigInt::from(i64::MAX)),
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
        // safe unwraps: radix and all strings are valid
        let order = match self.group_type {
            Prime => match self.data_type {
                F32 => match self.bound_type {
                    B0 => match self.model_type {
                        M3 => BigUint::from_str_radix("20_000_000_000_021", 10).unwrap(),
                        M6 => BigUint::from_str_radix("20_000_000_000_000_003", 10).unwrap(),
                        M9 => BigUint::from_str_radix("20_000_000_000_000_000_011", 10).unwrap(),
                        M12 => BigUint::from_str_radix("20_000_000_000_000_000_000_003", 10).unwrap(),
                    },
                    B2 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_000_000_000_000_021", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_000_000_000_000_000_057", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_000_000_000_000_000_000_069", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_000_000_000_000_000_000_000_003", 10).unwrap(),
                    },
                    B4 => match self.model_type {
                        M3 => BigUint::from_str_radix("200_000_000_000_000_003", 10).unwrap(),
                        M6 => BigUint::from_str_radix("200_000_000_000_000_000_089", 10).unwrap(),
                        M9 => BigUint::from_str_radix("200_000_000_000_000_000_000_069", 10).unwrap(),
                        M12 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_027", 10).unwrap(),
                    },
                    B6 => match self.model_type {
                        M3 => BigUint::from_str_radix("20_000_000_000_000_000_011", 10).unwrap(),
                        M6 => BigUint::from_str_radix("20_000_000_000_000_000_000_003", 10).unwrap(),
                        M9 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_009", 10).unwrap(),
                        M12 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_000_131", 10).unwrap(),
                    },
                    Bmax => match self.model_type {
                        M3 => BigUint::from_str_radix("680_564_700_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_281", 10).unwrap(),
                        M6 => BigUint::from_str_radix("680_564_700_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_323", 10).unwrap(),
                        M9 => BigUint::from_str_radix("680_564_700_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_191", 10).unwrap(),
                        M12 => BigUint::from_str_radix("680_564_700_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_000_083", 10).unwrap(),
                    },
                },
                F64 => match self.bound_type {
                    B0 => match self.model_type {
                        M3 => BigUint::from_str_radix("200_000_000_000_000_000_000_069", 10).unwrap(),
                        M6 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_027", 10).unwrap(),
                        M9 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_017", 10).unwrap(),
                        M12 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_000_159", 10).unwrap(),
                    },
                    B2 => match self.model_type {
                        M3 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_009", 10).unwrap(),
                        M6 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_000_131", 10).unwrap(),
                        M9 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_000_000_047", 10).unwrap(),
                        M12 => BigUint::from_str_radix("20_000_000_000_000_000_000_000_000_000_000_203", 10).unwrap(),
                    },
                    B4 => match self.model_type {
                        M3 => BigUint::from_str_radix("2_000_000_000_000_000_000_000_000_039", 10).unwrap(),
                        M6 => BigUint::from_str_radix("2_000_000_000_000_000_000_000_000_000_071", 10).unwrap(),
                        M9 => BigUint::from_str_radix("2_000_000_000_000_000_000_000_000_000_000_017", 10).unwrap(),
                        M12 => BigUint::from_str_radix("2_000_000_000_000_000_000_000_000_000_000_000_041", 10).unwrap(),
                    },
                    B6 => match self.model_type {
                        M3 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_017", 10).unwrap(),
                        M6 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_000_159", 10).unwrap(),
                        M9 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_000_000_003", 10).unwrap(),
                        M12 => BigUint::from_str_radix("200_000_000_000_000_000_000_000_000_000_000_000_023", 10).unwrap(),
                    },
                    Bmax => match self.model_type {
                        M3 => BigUint::from_str_radix("359_538_626_972_463_140_000_000_000_000_000_000_000_593_874_019_667_231_666_067_439_096_529_924_969_333_439_983_391_110_599_943_465_644_007_133_099_721_551_828_263_813_044_710_323_667_390_405_279_670_626_898_022_875_314_671_948_577_301_533_414_396_469_719_048_504_306_012_596_386_638_859_340_084_030_210_314_832_025_518_258_115_226_051_894_034_477_843_584_650_149_420_090_374_373_134_876_775_786_923_748_346_298_936_467_612_015_276_401_624_887_654_050_299_443_392_510_555_689_981_501_608_709_494_004_423_956_258_647_440_955_320_257_123_787_935_493_476_104_132_776_728_548_437_783_283_112_428_445_450_269_488_453_346_610_914_359_272_368_862_786_051_728_965_455_746_393_095_846_720_860_347_644_662_201_994_241_194_193_316_457_656_284_847_050_135_299_403_149_697_261_199_957_835_824_000_531_233_031_619_352_921_347_101_423_914_861_961_738_035_659_301", 10).unwrap(),
                        M6 => BigUint::from_str_radix("359_538_626_972_463_139_999_999_999_999_999_999_999_903_622_106_309_601_840_402_558_296_261_360_055_843_460_163_714_984_640_183_652_353_129_826_112_739_444_431_322_400_938_984_152_600_575_421_591_212_739_537_896_016_542_591_595_727_264_024_538_428_559_469_178_136_611_680_881_710_150_818_089_794_351_154_869_285_409_959_876_691_068_635_451_827_253_162_844_058_791_343_487_286_852_635_234_799_336_668_682_655_217_329_655_102_622_197_942_194_212_857_658_834_043_465_713_831_143_523_811_067_060_369_640_438_677_832_007_091_511_212_788_398_470_391_285_320_720_769_417_737_628_120_102_221_909_739_846_753_580_817_462_645_602_854_496_103_866_327_474_145_187_363_329_320_852_679_912_679_009_543_036_760_757_409_720_574_191_338_832_841_104_183_169_976_025_577_743_061_881_721_861_634_977_765_641_182_996_194_573_448_626_763_720_938_201_976_656_541_039_724_303", 10).unwrap(),
                        M9 => BigUint::from_str_radix("359_538_626_972_463_139_999_999_999_999_999_999_999_904_930_781_891_526_077_660_862_016_966_437_766_478_934_820_885_791_914_528_679_207_262_530_042_483_798_832_910_003_057_874_958_310_694_484_517_139_841_166_977_272_287_522_418_122_134_527_125_053_808_273_636_647_181_903_383_717_418_169_782_215_585_647_900_802_728_035_567_327_931_187_710_919_458_230_957_036_511_507_150_288_137_858_111_024_099_126_399_746_768_695_036_546_643_813_753_385_062_385_762_652_380_150_346_615_796_407_577_297_605_069_883_839_431_646_689_072_072_214_687_584_099_356_273_959_025_519_093_953_786_032_481_175_596_842_406_101_871_239_892_163_505_527_137_519_569_046_747_947_203_065_300_865_116_331_411_924_515_285_552_096_042_635_874_474_960_733_445_241_451_746_509_870_642_272_026_256_695_499_704_624_475_309_137_281_644_358_183_373_160_068_523_639_023_207_643_484_888_657_559_597", 10).unwrap(),
                        M12 => BigUint::from_str_radix("359_538_626_972_463_139_999_999_999_999_999_999_999_904_931_540_467_867_407_238_817_633_447_114_203_759_664_620_787_471_913_925_990_313_859_370_016_783_101_785_327_523_046_787_247_090_978_931_042_236_128_228_564_142_680_745_383_377_953_776_024_143_512_065_781_667_978_525_748_300_241_659_425_164_472_387_573_470_260_831_720_974_578_793_447_369_507_661_739_490_218_806_790_001_765_109_117_055_431_552_295_585_457_639_803_896_262_637_528_011_897_242_316_426_079_400_392_728_240_523_639_775_219_294_589_603_009_325_941_759_217_573_340_626_063_716_838_671_315_192_395_974_939_441_284_468_885_927_433_422_082_497_928_190_254_190_935_717_337_452_741_850_223_510_814_859_331_413_287_559_285_438_144_477_756_395_583_878_761_313_295_130_567_342_888_620_541_025_745_968_373_350_261_259_032_809_052_052_475_301_496_416_128_372_300_050_762_773_363_722_300_553_930_211_649", 10).unwrap(),
                    },
                },
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
        };
        MaskConfig {
            name,
            add_shift,
            exp_shift,
            order,
        }
    }
}
