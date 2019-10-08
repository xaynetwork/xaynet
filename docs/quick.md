# Quickstart

## Training

To execute a training session locally:

```shell
$ python -m xain.benchmark.exec \
    --group_name=abc \
    --task_name=def \
    --dataset=fashion-mnist-100p-iid-balanced \
    --model=blog_cnn \
    --R=2 \
    --E=2 \
    --C=0.02 \
    --B=64
```

## Benchmark Suites (using AWS EC2)

Here we describe how to configure and run an AWS service. Please bear in mind
that you are responsible for any costs that may arise when using these external
services.

### Configuration

In `~/.aws`, place two config files: `config` and `credentials`. Then execute
the command:

```shell
export AWS_PROFILE=xain-xain
```

### Running a benchmark suite

```shell
$ python -m xain.benchmark --benchmark_name=BENCHMARK_NAME
```

You can see valid benchmark names by using

```shell
$ python -m xain.benchmark --helpfull
```

### Connect to running instances

Start a benchmark suite

Then:

1. List all running EC2 instances:

```shell
$ AWS_PROFILE=xain-xain aws ec2 describe-instances  --filters Name=instance-state-code,Values=16 | jq '.Reservations[].Instances[] | "\(.Tags[].Value), \(.PublicIpAddress)"'
InstanceName1, "35.158.158.119"
OtherInstanceName2, "18.185.67.166"
```

2. Connect to one of the running instances using ssh:

```shell
$ ssh -i ~/.ssh/xain-ec2-remote-training.pem ubuntu@18.185.67.166
```

3. Follow the logs of the container to see how things are running:

```shell
$ docker logs -f $(docker ps -q)
```

## Plotting

To plot final task accuracies in a group of tasks use

```shell
$ pull_results
$ aggregate --group_name GROUP_NAME
```

### Removing obsolete plots

To remove a **single** benchmark result from S3:

```shell
aws s3 rm --recursive s3://xain-results/[group-name]
```

To remove **all** benchmark results from S3:

```shell
aws s3 rm --recursive s3://xain-results
```

## Ops

Package encapsulates most OPS related tasks.

### Local task

Run a task locally

```python
from xain.ops import docker, run

image_name = docker.build(should_push=True)
run.docker(image_name=image_name, benchmark_name="fashion-mnist-100p-iid-balanced")
```

### Remote task

Run a task on EC2

```python
from xain.ops import docker, run

image_name = docker.build(should_push=True)
run.ec2(
    image_name=image_name,
    timeout=20,
    benchmark_name="fashion-mnist-100p-iid-balanced",
)
```

## Datasets


This modules makes various public datasets available in federated dataset form.

You can find all public methods of the package in its `api` module.

**Example:**

```python
from xain.datasets.api import cifar10_random_splits_10_load_split
```
