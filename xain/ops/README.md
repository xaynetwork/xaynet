# Ops

Package encapsulates most OPS related tasks

## Local task

Run a task locally

```python
from xain.ops import docker, run

image_name = docker.build(should_push=True)
run.docker(image_name=image_name, benchmark_name="fashion-mnist-100p-iid-balanced")
```

## Remote task

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
