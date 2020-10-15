use super::{validate_pet, PetSettings, Settings};
use std::path::PathBuf;

#[test]
fn test_settings_new() {
    assert!(Settings::new(PathBuf::from("../../configs/config.toml")).is_ok());
    assert!(Settings::new(PathBuf::from("")).is_err());
}

#[test]
fn test_validate_pet() {
    assert!(validate_pet(&PetSettings::default()).is_ok());

    // phase times
    assert!(validate_pet(&PetSettings {
        min_sum_time: 2,
        max_sum_time: 1,
        ..PetSettings::default()
    })
    .is_err());
    assert!(validate_pet(&PetSettings {
        min_update_time: 2,
        max_update_time: 1,
        ..PetSettings::default()
    })
    .is_err());

    // fractions
    assert!(validate_pet(&PetSettings {
        sum: 0.,
        ..PetSettings::default()
    })
    .is_err());
    assert!(validate_pet(&PetSettings {
        sum: 1.,
        ..PetSettings::default()
    })
    .is_err());
    assert!(validate_pet(&PetSettings {
        update: 0.,
        ..PetSettings::default()
    })
    .is_err());
    assert!(validate_pet(&PetSettings {
        update: 1.,
        ..PetSettings::default()
    })
    .is_err());
}
