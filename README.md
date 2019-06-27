# AutoFL

AutoFL demonstrates automated architecture search in federated learning environments.

## Quick Start

### Clone Repository

```bash
$ git clone https://gitlab.com/xainag/autofl
$ cd autofl
```

### Verify Installation

AutoFL requires the following tools to be installed:

- [Python 3.6.8](https://python.org/)


Verify Python installation:

```bash
$ python3 --version
Python 3.6.8
```

## Packages

The `autofl` package contains the following sub-packages:

- `agent`: A reinforcement learning based agent which interacts with `flenv` using the OpenAI Gym interface. The agent samples architectures, trains them, and attempts to improve the performance of sampled architectures over time.
- `flenv`: Provides a reinforcement learning environment using OpenAI Gym. It receives architecture specification strings, uses them to build a `tf.keras.Model`, and then utilizes `fedml` to train the architecture in a federated fashion. A future version will also leverages a cache to hold weight matrices and speed up training (inspired by ENAS).
- `fedml`: Allows to train any `tf.keras.Model` in a federated fashion, i.e. by having a coordinator which manages the training across different participants who compute local updates on their individual partition of the data.
- `data`: Provides utilities to split existing datasets into shards in order to simulate a federated learning scenario. Other building blocks can be used to analyze, load, preprocess and augment the data partitions using `tf.data.Dataset`. Provided federated versions of popular vision datasets include:
  - CIFAR-10-F: A partitioned version of CIFAR-10
  - MNIST-F: A partitioned version of MNIST
