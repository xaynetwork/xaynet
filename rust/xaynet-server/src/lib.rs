#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(doc, forbid(warnings))]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/xaynetwork/xaynet/master/assets/logo.png",
    issue_tracker_base_url = "https://github.com/xaynetwork/xaynet/issues",
    html_favicon_url = "https://raw.githubusercontent.com/xaynetwork/xaynet/master/assets/favicon.png"
)]
//! `xaynet_server` is a backend for federated machile learning. It
//! ensures the users privacy using the _Privacy-Enhancing Technology_
//! (PET). Download the [whitepaper] for an introduction to the
//! protocol.
//!
//! [whitepaper]: https://uploads-ssl.webflow.com/5f0c5c0bb18a279f0a62919e/5f157004da6585f299fa542b_XayNet%20Whitepaper%202.1.pdf

pub mod examples;

pub mod metrics;
pub mod rest;
pub mod services;
pub mod settings;
pub mod state_machine;
pub mod storage;
