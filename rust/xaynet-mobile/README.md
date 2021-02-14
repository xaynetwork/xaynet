# Xaynet FFI

## Generate C-Header File

`ffi-support` provides some helpful macros to reduce boilerplate code.
However the feature (`--pretty=expanded`) to expand these macros
during the generation of the C-header file is still
unstable. Therefore we need to use the rust nightly.

To generate the header files, install [`cbindgen`] and run:

```
cargo build
RUSTUP_TOOLCHAIN=nightly cbindgen \
    --config cbindgen.toml \
    --crate xaynet-mobile \
    --output xaynet_ffi.h
```

## Run tests

```
cargo build
cargo test
```

To check for memory leaks, you can use Valgrind:


https://stackoverflow.com/questions/24745120/how-to-set-dynamic-link-library-path-and-environment-variable-for-a-process-in-v
```
cargo test
    Finished test [unoptimized + debuginfo] target(s) in 0.21s
     Running /Users/robert/projects/xain-fl/rust/target/debug/deps/xaynet_mobile-640128687334a8a4

valgrind --tool=memcheck  --leak-check=full --show-leak-kinds=all -s --trace-children=yes env INLINE_C_RS_CFLAGS="-I/Users/robert/projects/xain-fl/rust/xaynet-mobile -L/Users/robert/projects/xain-fl/rust/target/debug -D_DEBUG" INLINE_C_RS_LDFLAGS="/Users/robert/projects/xain-fl/rust/target/debug/libxaynet_mobile.dylib -framework Security -framework Foundation" ../target/debug/deps/xaynet_mobile-640128687334a8a4

valgrind ../target/debug/deps/xaynet_mobile-640128687334a8a4
```

macos big sur unsupported https://github.com/LouisBrunner/valgrind-macos/issues/21

[`cbindgen`]: https://github.com/eqrion/cbindgen/
