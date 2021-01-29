#![cfg_attr(doc, forbid(broken_intra_doc_links, private_intra_doc_links))]
#![doc(
    html_logo_url = "https://raw.githubusercontent.com/xaynetwork/xaynet/master/assets/xaynet_banner.png",
    html_favicon_url = "https://raw.githubusercontent.com/xaynetwork/xaynet/master/assets/favicon.png",
    issue_tracker_base_url = "https://github.com/xaynetwork/xaynet/issues"
)]
//! This crate containes the Rust component of Federated Analytics,
//! a framework that allows mobile applications to collect and aggregate
//! analytics data via the _Privacy-Enhancing Technology_ (PET) protocol.

pub mod data_combination;
pub mod database;
