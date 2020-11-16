use std::path::PathBuf;
#[cfg(feature = "tls")]
use std::{net::SocketAddr, str::FromStr};

#[cfg(feature = "tls")]
use super::{validate_api, ApiSettings};
use super::{validate_pet, PetSettings, Settings};

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

#[cfg(feature = "tls")]
#[test]
fn test_validate_api() {
    let bind_address = SocketAddr::from_str("0.0.0.0:0000").unwrap();

    assert!(validate_api(&ApiSettings {
        bind_address,
        tls_certificate: Some(PathBuf::new()),
        tls_key: Some(PathBuf::new()),
        tls_client_auth: Some(PathBuf::new()),
    })
    .is_ok());
    assert!(validate_api(&ApiSettings {
        bind_address,
        tls_certificate: Some(PathBuf::new()),
        tls_key: Some(PathBuf::new()),
        tls_client_auth: None,
    })
    .is_ok());
    assert!(validate_api(&ApiSettings {
        bind_address,
        tls_certificate: None,
        tls_key: None,
        tls_client_auth: Some(PathBuf::new()),
    })
    .is_ok());

    assert!(validate_api(&ApiSettings {
        bind_address,
        tls_certificate: Some(PathBuf::new()),
        tls_key: None,
        tls_client_auth: Some(PathBuf::new()),
    })
    .is_err());
    assert!(validate_api(&ApiSettings {
        bind_address,
        tls_certificate: None,
        tls_key: Some(PathBuf::new()),
        tls_client_auth: Some(PathBuf::new()),
    })
    .is_err());
    assert!(validate_api(&ApiSettings {
        bind_address,
        tls_certificate: Some(PathBuf::new()),
        tls_key: None,
        tls_client_auth: None,
    })
    .is_err());
    assert!(validate_api(&ApiSettings {
        bind_address,
        tls_certificate: None,
        tls_key: Some(PathBuf::new()),
        tls_client_auth: None,
    })
    .is_err());
    assert!(validate_api(&ApiSettings {
        bind_address,
        tls_certificate: None,
        tls_key: None,
        tls_client_auth: None,
    })
    .is_err());
}
