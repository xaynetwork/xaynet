# XAIN

XAIN demonstrates federated learning on various data partitioning schemes.

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

## Related Papers

- [Communication-Efficient Learning of Deep Networks from Decentralized Data](https://arxiv.org/abs/1602.05629)
- [Analyzing Federated Learning through an Adversarial Lens](https://arxiv.org/abs/1811.12470)
- [Towards Federated Learning at Scale: System Design](https://arxiv.org/abs/1902.01046)
