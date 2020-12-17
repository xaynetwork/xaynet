#[cfg(feature = "tls")]
use std::{net::SocketAddr, path::PathBuf, str::FromStr};

#[cfg(feature = "tls")]
use super::ApiSettings;
use super::{PetSettings, Settings};

#[test]
fn test_settings_new() {
    assert!(Settings::new("../../configs/config.toml").is_ok());
    assert!(Settings::new("").is_err());
}

#[test]
fn test_validate_pet() {
    assert!(PetSettings::default().validate_pet().is_ok());

    // phase times
    assert!(PetSettings {
        min_sum_time: 2,
        max_sum_time: 1,
        ..PetSettings::default()
    }
    .validate_pet()
    .is_err());
    assert!(PetSettings {
        min_update_time: 2,
        max_update_time: 1,
        ..PetSettings::default()
    }
    .validate_pet()
    .is_err());

    // fractions
    assert!(PetSettings {
        sum: 0.,
        ..PetSettings::default()
    }
    .validate_pet()
    .is_err());
    assert!(PetSettings {
        sum: 1.,
        ..PetSettings::default()
    }
    .validate_pet()
    .is_err());
    assert!(PetSettings {
        update: 0.,
        ..PetSettings::default()
    }
    .validate_pet()
    .is_err());
    assert!(PetSettings {
        update: 1. + f64::EPSILON,
        ..PetSettings::default()
    }
    .validate_pet()
    .is_err());
}

#[cfg(feature = "tls")]
#[test]
fn test_validate_api() {
    let bind_address = SocketAddr::from_str("0.0.0.0:0000").unwrap();

    assert!(ApiSettings {
        bind_address,
        tls_certificate: Some(PathBuf::new()),
        tls_key: Some(PathBuf::new()),
        tls_client_auth: Some(PathBuf::new()),
    }
    .validate_api()
    .is_ok());
    assert!(ApiSettings {
        bind_address,
        tls_certificate: Some(PathBuf::new()),
        tls_key: Some(PathBuf::new()),
        tls_client_auth: None,
    }
    .validate_api()
    .is_ok());
    assert!(ApiSettings {
        bind_address,
        tls_certificate: None,
        tls_key: None,
        tls_client_auth: Some(PathBuf::new()),
    }
    .validate_api()
    .is_ok());

    assert!(ApiSettings {
        bind_address,
        tls_certificate: Some(PathBuf::new()),
        tls_key: None,
        tls_client_auth: Some(PathBuf::new()),
    }
    .validate_api()
    .is_err());
    assert!(ApiSettings {
        bind_address,
        tls_certificate: None,
        tls_key: Some(PathBuf::new()),
        tls_client_auth: Some(PathBuf::new()),
    }
    .validate_api()
    .is_err());
    assert!(ApiSettings {
        bind_address,
        tls_certificate: Some(PathBuf::new()),
        tls_key: None,
        tls_client_auth: None,
    }
    .validate_api()
    .is_err());
    assert!(ApiSettings {
        bind_address,
        tls_certificate: None,
        tls_key: Some(PathBuf::new()),
        tls_client_auth: None,
    }
    .validate_api()
    .is_err());
    assert!(ApiSettings {
        bind_address,
        tls_certificate: None,
        tls_key: None,
        tls_client_auth: None,
    }
    .validate_api()
    .is_err());
}
