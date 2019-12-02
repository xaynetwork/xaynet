[![CircleCI](https://img.shields.io/circleci/build/github/xainag/xain-fl/master?style=flat-square)](https://circleci.com/gh/xainag/xain-fl/tree/master)
[![PyPI](https://img.shields.io/pypi/v/xain-fl?style=flat-square)](https://pypi.org/project/xain-fl/)
[![GitHub license](https://img.shields.io/github/license/xainag/xain-fl?style=flat-square)](https://github.com/xainag/xain-fl/blob/master/LICENSE)

# XAIN

The XAIN project is building a GDPR-compliance layer for machine learning. The approach relies on federated machine learning (FedML) as enabling technology that removes compliance-related adoption barriers of AI applications used in production.

At present, the source code in this project demonstrates the effectiveness of our FedML implementation on well known benchmarks using a realistic deep learning model structure. We will soon add a link to details on those experiments.

In the future, we will open source here a first minimal viable product for this layer. And we will add links to articles and papers that describe our approaches to networking, architecture, and privacy-preserving technology. We will also provide references to legal opinions about how and why our compliance layer for machine learning meets the demands of GDPR.

POLITE NOTE: We want to point out that running the benchmarks as described below is consuming considerable resources. XAIN cannot take any responsibilities for costs that arise for you when you execute these demanding machine-learning benchmarks.

## Quick Start

XAIN requires [Python 3.6+](https://python.org/). To install the `xain-fl` package just run:

```shell
$ python -m pip install xain-fl
```

XAIN can also be installed with GPU support through the `gpu` extra feature. To
install the `xain-fl` package with support for GPUs just run:

```shell
$ python -m pip install xain-fl[gpu]
```

### Running training sessions and benchmarks

To run training sessions, see the [benchmark
package](https://github.com/xainag/xain-fl/tree/master/benchmarks/benchmark) and the
[benchmark
documentation](https://github.com/xainag/xain-fl/blob/master/docs/quick.md#training).

## Install from source

For development we require some extra system dependencies:

- [clang-format 8+](https://clang.llvm.org/docs/ClangFormat.html)
  - Linux: `sudo apt install clang-format`
  - macOS: `brew install clang-format`

### Clone Repository & Install XAIN in development mode

To clone this repository and to install the XAIN project, please execute the following commands:

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

## Related Papers and Articles

- [An introduction to XAIN’s GDPR-compliance Layer for Machine Learning](https://medium.com/xain/an-introduction-to-xains-gdpr-compliance-layer-for-machine-learning-f7c321b31b06)
- [Communication-Efficient Learning of Deep Networks from Decentralized Data](https://arxiv.org/abs/1602.05629)
- [Analyzing Federated Learning through an Adversarial Lens](https://arxiv.org/abs/1811.12470)
- [Towards Federated Learning at Scale: System Design](https://arxiv.org/abs/1902.01046)
