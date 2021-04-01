use std::{env, fs::read_dir, path::PathBuf};

use cbindgen::{Builder, Config};

fn cargo_rerun_if_changed(dir: PathBuf) {
    for entry in read_dir(dir).expect("Failed to read dir.") {
        let entry = entry.expect("Failed to read entry.").path();
        if entry.is_dir() {
            cargo_rerun_if_changed(entry);
        } else {
            println!("cargo:rerun-if-changed={}", entry.display());
        }
    }
}

fn main() {
    let crate_dir = PathBuf::from(
        env::var("CARGO_MANIFEST_DIR").expect("Failed to read CARGO_MANIFEST_DIR env."),
    );
    let bind_config = crate_dir.join("cbindgen.toml");
    let bind_file = crate_dir.join("xaynet_ffi.h");

    // cargo doesn't check directories recursively so we have to do it by hand, also emitting a
    // rerun-if line cancels the default rerun for changes in the crate directory
    cargo_rerun_if_changed(crate_dir.join("src"));
    println!(
        "cargo:rerun-if-changed={}",
        crate_dir.join("Cargo.toml").display(),
    );
    println!("cargo:rerun-if-changed={}", bind_config.display());

    let config = Config::from_file(bind_config).expect("Failed to read config.");
    Builder::new()
        .with_crate(crate_dir)
        .with_config(config)
        .generate()
        .expect("Failed to generate bindings.")
        .write_to_file(bind_file);
}
