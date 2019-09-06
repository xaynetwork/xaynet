import os
from tempfile import TemporaryDirectory
from time import strftime
from typing import Dict, List, Optional

from absl import flags, logging

from xain.helpers import storage
from xain.ops import docker, results, run

from .task import Task, UnitaryVisionTask, VisionTask

FLAGS = flags.FLAGS


class Benchmark:
    def __init__(self, tasks: List[Task], aggregation_name: str):
        self.tasks = tasks
        self.aggregation_name = aggregation_name


benchmarks: Dict[str, Benchmark] = {
    #
    # ##############################
    # Federated Versus Unitary
    # ##############################
    #
    "flul-fashion-mnist-100p-iid-balanced": Benchmark(
        tasks=[
            UnitaryVisionTask(
                dataset_name="fashion-mnist-100p-iid-balanced",
                partition_id=0,
                instance_cores=4,
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-iid-balanced", instance_cores=8
            ),
        ],
        aggregation_name="flul-aggregation",
    ),
    "flul-fashion-mnist-100p-noniid-02cpp": Benchmark(
        tasks=[
            UnitaryVisionTask(
                dataset_name="fashion-mnist-100p-noniid-02cpp",
                partition_id=0,
                instance_cores=4,
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-02cpp", instance_cores=8
            ),
        ],
        aggregation_name="flul-aggregation",
    ),
    "flul-cifar-10-100p-iid-balanced": Benchmark(
        tasks=[
            UnitaryVisionTask(
                dataset_name="cifar-10-100p-iid-balanced",
                model_name="resnet20",
                partition_id=0,
                instance_cores=16,
            ),
            VisionTask(
                dataset_name="cifar-10-100p-iid-balanced",
                model_name="resnet20",
                instance_cores=32,
            ),
        ],
        aggregation_name="flul-aggregation",
    ),
    "flul-cifar-10-100p-noniid-02cpp": Benchmark(
        tasks=[
            UnitaryVisionTask(
                dataset_name="cifar-10-100p-noniid-02cpp",
                model_name="resnet20",
                partition_id=0,
                instance_cores=16,
            ),
            VisionTask(
                dataset_name="cifar-10-100p-noniid-02cpp",
                model_name="resnet20",
                instance_cores=32,
            ),
        ],
        aggregation_name="flul-aggregation",
    ),
    "flul-fashion-mnist-100p-b-1-045": Benchmark(
        tasks=[
            UnitaryVisionTask(
                dataset_name="fashion-mnist-100p-b-1-045",
                partition_id=0,
                instance_cores=4,
            ),
            UnitaryVisionTask(
                dataset_name="fashion-mnist-100p-b-1-045",
                partition_id=99,
                instance_cores=4,
            ),
            VisionTask(dataset_name="fashion-mnist-100p-b-1-045", instance_cores=8),
        ],
        aggregation_name="flul-aggregation",
    ),
    "flul-cifar-10-100p-b-1-045": Benchmark(
        tasks=[
            UnitaryVisionTask(
                dataset_name="cifar-10-100p-b-1-045", partition_id=0, instance_cores=4
            ),
            UnitaryVisionTask(
                dataset_name="cifar-10-100p-b-1-045", partition_id=99, instance_cores=4
            ),
            VisionTask(dataset_name="cifar-10-100p-b-1-045", instance_cores=8),
        ],
        aggregation_name="flul-aggregation",
    ),
    #
    # ##############################
    # Class Partitioning
    # ##############################
    #
    "cpp-fashion-mnist-100p": Benchmark(
        tasks=[
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-01cpp", instance_cores=4
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-02cpp", instance_cores=4
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-03cpp", instance_cores=4
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-04cpp", instance_cores=4
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-05cpp", instance_cores=4
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-06cpp", instance_cores=4
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-07cpp", instance_cores=4
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-08cpp", instance_cores=4
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-09cpp", instance_cores=4
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-iid-balanced", instance_cores=4
            ),
            UnitaryVisionTask(
                dataset_name="fashion-mnist-100p-noniid-01cpp", instance_cores=4
            ),
            UnitaryVisionTask(
                dataset_name="fashion-mnist-100p-noniid-02cpp", instance_cores=4
            ),
            UnitaryVisionTask(
                dataset_name="fashion-mnist-100p-noniid-03cpp", instance_cores=4
            ),
            UnitaryVisionTask(
                dataset_name="fashion-mnist-100p-noniid-04cpp", instance_cores=4
            ),
            UnitaryVisionTask(
                dataset_name="fashion-mnist-100p-noniid-05cpp", instance_cores=4
            ),
            UnitaryVisionTask(
                dataset_name="fashion-mnist-100p-noniid-06cpp", instance_cores=4
            ),
            UnitaryVisionTask(
                dataset_name="fashion-mnist-100p-noniid-07cpp", instance_cores=4
            ),
            UnitaryVisionTask(
                dataset_name="fashion-mnist-100p-noniid-08cpp", instance_cores=4
            ),
            UnitaryVisionTask(
                dataset_name="fashion-mnist-100p-noniid-09cpp", instance_cores=4
            ),
            UnitaryVisionTask(
                dataset_name="fashion-mnist-100p-iid-balanced", instance_cores=4
            ),
        ],
        aggregation_name="cpp-aggregation",
    ),
    #
    # ##############################
    # Volume Partitioning
    # ##############################
    #
    "vol-fashion-mnist-100p": Benchmark(
        tasks=[
            VisionTask(dataset_name="fashion-mnist-100p-b-1-000", instance_cores=4),
            VisionTask(dataset_name="fashion-mnist-100p-b-1-005", instance_cores=4),
            VisionTask(dataset_name="fashion-mnist-100p-b-1-010", instance_cores=4),
            VisionTask(dataset_name="fashion-mnist-100p-b-1-015", instance_cores=4),
            VisionTask(dataset_name="fashion-mnist-100p-b-1-020", instance_cores=4),
            VisionTask(dataset_name="fashion-mnist-100p-b-1-025", instance_cores=4),
            VisionTask(dataset_name="fashion-mnist-100p-b-1-030", instance_cores=4),
            VisionTask(dataset_name="fashion-mnist-100p-b-1-035", instance_cores=4),
            VisionTask(dataset_name="fashion-mnist-100p-b-1-040", instance_cores=4),
            VisionTask(dataset_name="fashion-mnist-100p-b-1-045", instance_cores=4),
        ],
        aggregation_name="vol-aggregation",
    ),
    "vol-cifar-10-100p": Benchmark(
        tasks=[
            VisionTask(dataset_name="cifar-10-100p-b-1-000", instance_cores=16),
            VisionTask(dataset_name="cifar-10-100p-b-1-005", instance_cores=16),
            VisionTask(dataset_name="cifar-10-100p-b-1-010", instance_cores=16),
            VisionTask(dataset_name="cifar-10-100p-b-1-015", instance_cores=16),
            VisionTask(dataset_name="cifar-10-100p-b-1-020", instance_cores=16),
            VisionTask(dataset_name="cifar-10-100p-b-1-025", instance_cores=16),
            VisionTask(dataset_name="cifar-10-100p-b-1-030", instance_cores=16),
            VisionTask(dataset_name="cifar-10-100p-b-1-035", instance_cores=16),
            VisionTask(dataset_name="cifar-10-100p-b-1-040", instance_cores=16),
            VisionTask(dataset_name="cifar-10-100p-b-1-045", instance_cores=16),
        ],
        aggregation_name="vol-aggregation",
    ),
    #
    # ##############################
    # E: Small to high
    # ##############################
    #
    "E-fashion-mnist-100p": Benchmark(
        tasks=[
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-05cpp",
                R=256,
                E=1,
                instance_cores=8,
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-05cpp",
                R=128,
                E=2,
                instance_cores=8,
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-05cpp",
                R=64,
                E=4,
                instance_cores=8,
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-05cpp",
                R=32,
                E=8,
                instance_cores=8,
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-05cpp",
                R=16,
                E=16,
                instance_cores=8,
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-05cpp",
                R=8,
                E=32,
                instance_cores=8,
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-05cpp",
                R=4,
                E=64,
                instance_cores=8,
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-05cpp",
                R=2,
                E=128,
                instance_cores=8,
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-05cpp",
                R=1,
                E=256,
                instance_cores=8,
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-05cpp",
                R=256,
                E=1,
                instance_cores=8,
            ),
        ],
        aggregation_name="cpp-aggregation",
    ),
    #
    # ##############################
    # End-To-End Tests
    # ##############################
    #
    "e2e-flul-fashion-mnist": Benchmark(
        tasks=[
            UnitaryVisionTask(
                dataset_name="fashion-mnist-100p-noniid-02cpp",
                E=4,
                partition_id=0,
                instance_cores=4,
                timeout=10,
            ),
            VisionTask(
                dataset_name="fashion-mnist-100p-noniid-02cpp",
                R=2,
                E=2,
                C=0.02,
                instance_cores=4,
                timeout=10,
            ),
        ],
        aggregation_name="flul-aggregation",
    ),
    "e2e-flul-cifar-10": Benchmark(
        tasks=[
            UnitaryVisionTask(
                dataset_name="cifar-10-100p-noniid-02cpp",
                E=4,
                partition_id=0,
                instance_cores=16,
                timeout=10,
            ),
            VisionTask(
                dataset_name="cifar-10-100p-noniid-02cpp",
                R=2,
                E=2,
                C=0.02,
                instance_cores=16,
                timeout=10,
            ),
        ],
        aggregation_name="flul-aggregation",
    ),
}


def build_task_name(task):
    return "_".join(
        [
            task.__class__.__name__,
            task.dataset_name,
            task.model_name,
            str(task.R),
            str(task.E),
            str(task.C),
            str(task.B),
        ]
    )


def run_benchmark(benchmark_name: str):
    logging.info(f"Building Docker image for benchmark {benchmark_name}")
    docker_image_name = docker.build(should_push=True)

    logging.info(f"Starting benchmark {benchmark_name}")
    benchmark = benchmarks[benchmark_name]
    group_name = f"group_{benchmark_name}_{strftime('%Y%m%dT%H%M')}"

    # TODO Initiate tasks in parallel
    for task in benchmark.tasks:
        model_name = task.model_name
        dataset_name = task.dataset_name
        run_task(
            docker_image_name=docker_image_name,
            group_name=group_name,
            task_name=build_task_name(task),
            model=model_name,
            dataset=dataset_name,
            R=task.R,
            E=task.E,
            C=task.C,
            B=task.B,
            partition_id=task.partition_id,
            instance_cores=task.instance_cores,
            timeout=task.timeout,
        )

    with TemporaryDirectory() as tmpdir:
        fname = os.path.join(tmpdir, "config.json")
        data = {"aggregation_name": benchmark.aggregation_name}
        storage.write_json(data, fname)
        results.push(group_name=group_name, task_name="", output_dir=tmpdir)


def run_task(
    docker_image_name: str,
    group_name: str,
    task_name: str,
    model: str,
    dataset: str,
    R: int,
    E: int,
    C: float,
    B: int,
    partition_id: Optional[int],
    instance_cores: int,
    timeout: int,
):
    task_msg = f"{model}, {dataset}, {R}, {E}, {C}, {B}, {instance_cores}, {timeout}"
    logging.info(f"Attempting to run task on EC2: {task_msg}")
    run.ec2(
        image=docker_image_name,
        timeout=timeout,
        instance_cores=instance_cores,
        # The following arguments will be passed as absl flags:
        group_name=group_name,
        task_name=task_name,
        model=model,
        dataset=dataset,
        R=R,
        E=E,
        C=C,
        B=B,
        partition_id=partition_id,
    )


def main(_):
    benchmark_name = FLAGS.benchmark_name
    assert benchmark_name in benchmarks
    run_benchmark(benchmark_name=benchmark_name)
