# XAIN

The XAIN project is building a GDPR-compliance layer for machine learning. The approach relies on federated machine learning (FedML) as enabling technology that removes compliance-related adoption barriers of AI applications used in production.

At present, the source code in this project demonstrates the effectiveness of our FedML implementation on well known benchmarks using a realistic deep learning model structure. We will soon add a link to details on those experiments.

In the future, we will open source here a first minimal viable product for this layer. And we will add links to articles and papers that describe our approaches to networking, architcture, and privacy-perserving technology. We will also provide references to legal opinions about how and why our compliance layer for machine learning meets the demands of GDPR.

POLITE NOTE: We want to point out that running the benchmarks as described below is consuming considerable resources. XAIN cannot take any responsibilities for costs that arise for you when you execute these demanding machine-learning benchmarks.

## Quick Start

XAIN requires the following tools to be installed:

- [Python 3.6+](https://python.org/)
- clang-format (on osx: brew install clang-format)

### Clone Repository & Install XAIN

To clone this repository and to install the XAIN project, please execute the following commands:

```shell
$ git clone https://github.com/xainag/xain.git
$ cd xain

$ pip install -e .[dev,cpu]
```

### Verify Installation

You can verify the installation by running the tests

```shell
$ pytest
```

### Running training sessions and benchmarks

To run training sessions, see the [benchmark package](https://github.com/xainag/xain/tree/master/xain/benchmark)

## Related Papers and Articles

- [An introduction to XAIN’s GDPR-compliance Layer for Machine Learning](https://medium.com/xain/an-introduction-to-xains-gdpr-compliance-layer-for-machine-learning-f7c321b31b06)
- [Communication-Efficient Learning of Deep Networks from Decentralized Data](https://arxiv.org/abs/1602.05629)
- [Analyzing Federated Learning through an Adversarial Lens](https://arxiv.org/abs/1811.12470)
- [Towards Federated Learning at Scale: System Design](https://arxiv.org/abs/1902.01046)
