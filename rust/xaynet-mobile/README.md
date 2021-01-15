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

### macOS

```
cc -o tests/ffi_test.o -Wl,-dead_strip -I. tests/ffi_test.c ../target/debug/libxaynet_mobile.a -framework Security -framework Foundation
./tests/ffi_test.o
```

### Linux

```
gcc \
    tests/ffi_test.c
    -Wall \
    -I. \
    -lpthread -lm -ldl \
    ../target/debug/libxaynet_mobile.a \
    -o tests/ffi_test.o
./tests/ffi_test.o
```

To check for memory leaks, you can use Valgrind:

```
valgrind --tool=memcheck  --leak-check=full --show-leak-kinds=all -s ./tests/ffi_test.o
```

[`cbindgen`]: https://github.com/eqrion/cbindgen/
