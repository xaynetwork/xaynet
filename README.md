# AutoFL

AutoFL demonstrates federated learning on various data partitioning schemes.

## Quick Start

### Clone Repository

```bash
$ git clone https://gitlab.com/xainag/autofl
$ cd autofl
```

### Verify Installation

AutoFL requires the following tools to be installed:

- [Python 3.6.9](https://python.org/)

Verify Python installation:

```bash
$ python3 --version
Python 3.6.9
```

## AWS

### Configuration

In `~/.aws`, place two config files: `config` and `credentials`. Then:

```bash
export AWS_PROFILE=xain-autofl
```

### Connect to running instances

After starting a training job on AWS using `./scripts/train_remote.sh`:

1. List all running EC2 instances:
   ```shell
   $ AWS_PROFILE=xain-autofl aws ec2 describe-instances  --filters Name=instance-state-code,Values=16 | jq '.Reservations[].Instances[].PublicIpAddress'
   "35.158.158.119"
   "18.185.67.166"
   ```
2. Connect to one of the running instances using ssh:

   ```shell
   $ ssh -i ~/.ssh/autofl_job.pem.xain ubuntu@18.185.67.166
   ```

3. Follow the logs of the container:
   ```shell
   $ docker logs -f $(docker ps -q)
   ```

## Packages

The `autofl` package contains the following sub-packages:

- `agent`: A reinforcement learning based agent which interacts with `flenv` using the OpenAI Gym interface. The agent samples architectures, trains them, and attempts to improve the performance of sampled architectures over time.
- `flenv`: Provides a reinforcement learning environment using OpenAI Gym. It receives architecture specification strings, uses them to build a `tf.keras.Model`, and then utilizes `fedml` to train the architecture in a federated fashion. A future version will also leverages a cache to hold weight matrices and speed up training (inspired by ENAS).
- `fedml`: Allows to train any `tf.keras.Model` in a federated fashion, i.e. by having a coordinator which manages the training across different participants who compute local updates on their individual partition of the data.
- `data`: Provides utilities to split existing datasets into shards in order to simulate a federated learning scenario. Other building blocks can be used to analyze, load, preprocess and augment the data partitions using `tf.data.Dataset`. Provided federated versions of popular vision datasets include:
  - CIFAR-10-F: A partitioned version of CIFAR-10
  - MNIST-F: A partitioned version of MNIST

## Related Papers

- [Communication-Efficient Learning of Deep Networks from Decentralized Data](https://arxiv.org/abs/1602.05629)
- [Analyzing Federated Learning through an Adversarial Lens](https://arxiv.org/abs/1811.12470)
- [Towards Federated Learning at Scale: System Design](https://arxiv.org/abs/1902.01046)

## PyTorch

PyTorch might require the following native library on macOS:

`brew install libomp`
