# Benchmark Execution

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

## Benchmark Suits (using AWS EC2)

Here we describe how to configure and run an AWS service. Please bear in mind that you are responsible for any costs that may arise when using these external services.

### Configuration

In `~/.aws`, place two config files: `config` and `credentials`. Then execute the command:

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

Start a benchmark suit

Then:

1. List all running EC2 instances:

```shell
$ AWS_PROFILE=xain-xain aws ec2 describe-instances  --filters Name=instance-state-code,Values=16 | jq '.Reservations[].Instances[].PublicIpAddress'
"35.158.158.119"
"18.185.67.166"
```

2. Connect to one of the running instances using ssh:

```shell
$ ssh -i ~/.ssh/autofl_job.pem.xain ubuntu@18.185.67.166
```

3. Follow the logs of the container to see how things are running:

```shell
$ docker logs -f $(docker ps -q)
```

# Plotting

To plot final task accuracies in a group of tasks use

```shell
$ pull_results
$ plot_final_task_accuracies --group_name=GROUP_NAME
```
