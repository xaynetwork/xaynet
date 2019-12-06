# Installation of XAIN

XAIN FL requires [Python 3.6+](https://python.org/).

## Install from PyPi package

To install the `xain-fl` package just run:

```shell
$ python -m pip install xain-fl
```

XAIN FL can also be installed with GPU support through the `gpu` extra feature. To
install the `xain-fl` package with support for GPUs just run:

```shell
$ python -m pip install xain-fl[gpu]
```

## Install from source

For development we require some extra system dependencies:

- [clang-format 8+](https://clang.llvm.org/docs/ClangFormat.html)
  - Linux: `sudo apt install clang-format`
  - macOS: `brew install clang-format`

### Clone Repository & Install XAIN FL in development mode

To clone this repository and to install the XAIN FL project, please execute the following commands:

```shell
$ git clone https://github.com/xainag/xain-fl.git
$ cd xain-fl

$ python -m pip install -e .[dev]
```

### Verify Installation

You can verify the installation by running the tests

```shell
$ pytest
```

### Building the Documentation

The project documentation resides under `docs/`. To build the documentation
run:

```shell
$ cd docs/
$ make docs
```

The generated documentation will be under `docs/_build/html/`. You can open the
root of the documentation by opening `docs/_build/html/index.html` on your
favorite browser.
