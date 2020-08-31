# Xaynet FFI

## Generate C-Header File

`ffi-support` provides some helpful macros to reduce boilerplate code.
However the feature (`--pretty=expanded`) to expand these macros during the generation of the
C-header file is still unstable. Therefore we need to use the rust nightly.

- run `RUSTUP_TOOLCHAIN=nightly cbindgen --output xaynet_ffi.h`


## Run tests

### macOS

- run `cargo build`
- run `cc -o tests/ffi_test.o -Wl,-dead_strip -I. tests/ffi_test.c ../target/debug/libxaynet_sdk.a -framework Security -framework Foundation`
- run `./tests/ffi_test.o`