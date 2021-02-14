use std::{env, path::PathBuf};

fn main() {
    let include_dir = env::var("CARGO_MANIFEST_DIR").unwrap();

    let mut shared_object_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    shared_object_dir.push("..");
    shared_object_dir.push("target");
    shared_object_dir.push(env::var("PROFILE").unwrap());
    let shared_object_dir = shared_object_dir.as_path().to_string_lossy();

    // The following options mean:
    //
    // * `-I`, add `include_dir` to include search path,
    // * `-L`, add `shared_object_dir` to library search path,
    // * `-D_DEBUG`, enable debug mode to enable `assert.h`.
    println!(
        "cargo:rustc-env=INLINE_C_RS_CFLAGS=-I{I} -L{L} -D_DEBUG",
        I = include_dir,
        L = shared_object_dir.clone(),
    );

    // Here, we pass the fullpath to the shared object with
    // `LDFLAGS`.
    println!(
        "cargo:rustc-env=INLINE_C_RS_LDFLAGS={shared_object_dir}/{lib}",
        shared_object_dir = shared_object_dir,
        lib = if cfg!(target_os = "windows") {
            panic!("windows is unsupported")
        } else if cfg!(target_os = "macos") {
            "libxaynet_mobile.dylib -framework Security -framework Foundation".to_string()
        } else {
            "libxaynet_mobile.so".to_string()
        }
    );
}
