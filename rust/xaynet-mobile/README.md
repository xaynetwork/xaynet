# Xaynet FFI

## Generate C-Header File

To generate the header files, run `cargo build`.


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
    -pthread -Wl,--no-as-needed -lm -ldl \
    ../target/debug/libxaynet_mobile.a \
    -o tests/ffi_test.o
./tests/ffi_test.o
```

To check for memory leaks, you can use Valgrind:

```
valgrind --tool=memcheck  --leak-check=full --show-leak-kinds=all -s ./tests/ffi_test.o
```
