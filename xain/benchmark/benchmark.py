from time import strftime
from typing import Callable, Dict, List

from absl import flags, logging

from xain.ops import docker, run

from .aggregation import cpp_aggregation, flul_aggregation
from .task import FashionMNISTTask, Task, UnitaryFashionMNISTTask

FLAGS = flags.FLAGS


aggregations: Dict[str, Callable] = {
    "flul-aggregation": flul_aggregation,
    "cpp-aggregation": cpp_aggregation,
}


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
            UnitaryFashionMNISTTask(
                dataset_name="fashion-mnist-100p-iid-balanced", partition_id=0
            ),
            FashionMNISTTask(dataset_name="fashion-mnist-100p-iid-balanced"),
        ],
        aggregation_name="flul-aggregation",
    ),
    "flul-fashion-mnist-100p-noniid-02cpp": Benchmark(
        tasks=[
            UnitaryFashionMNISTTask(
                dataset_name="fashion-mnist-100p-noniid-02cpp", partition_id=0
            ),
            FashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-02cpp"),
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
            FashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-01cpp"),
            FashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-02cpp"),
            FashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-03cpp"),
            FashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-04cpp"),
            FashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-05cpp"),
            FashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-06cpp"),
            FashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-07cpp"),
            FashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-08cpp"),
            FashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-09cpp"),
            FashionMNISTTask(dataset_name="fashion-mnist-100p-iid-balanced"),
            UnitaryFashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-01cpp"),
            UnitaryFashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-02cpp"),
            UnitaryFashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-03cpp"),
            UnitaryFashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-04cpp"),
            UnitaryFashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-05cpp"),
            UnitaryFashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-06cpp"),
            UnitaryFashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-07cpp"),
            UnitaryFashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-08cpp"),
            UnitaryFashionMNISTTask(dataset_name="fashion-mnist-100p-noniid-09cpp"),
            UnitaryFashionMNISTTask(dataset_name="fashion-mnist-100p-iid-balanced"),
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
            UnitaryFashionMNISTTask(
                dataset_name="fashion-mnist-100p-noniid-02cpp", E=4, partition_id=0
            ),
            FashionMNISTTask(
                dataset_name="fashion-mnist-100p-noniid-02cpp", R=2, E=2, C=0.02
            ),
        ],
        aggregation_name="flul-aggregation",
    ),
    "e2e-flul-cifar-10": Benchmark(
        tasks=[
            UnitaryFashionMNISTTask(
                dataset_name="cifar-10-100p-noniid-02cpp", E=4, partition_id=0
            ),
            FashionMNISTTask(
                dataset_name="cifar-10-100p-noniid-02cpp", R=2, E=2, C=0.02
            ),
        ],
        aggregation_name="flul-aggregation",
    ),
}


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
            task_class=task.__class__.__name__,
            model=model_name,
            dataset=dataset_name,
            R=task.R,
            E=task.E,
            C=task.C,
            B=task.B,
        )

    # Aggregate results
    # TODO wait for completion or move to separate task
    aggregation_fn = aggregations[benchmark.aggregation_name]
    aggregation_fn()


def run_task(
    docker_image_name: str,
    group_name: str,
    task_class: str,
    model: str,
    dataset: str,
    R: int,
    E: int,
    C: float,
    B: int,
):
    logging.info(
        f"Attempting to run task on EC2: {model}, {dataset}, {R}, {E}, {C}, {B}"
    )
    run.ec2(
        image=docker_image_name,
        timeout=300,  # TODO dynamic from benchmark config
        instance_cores=2,  # TODO dynamic from benchmark config
        # The following arguments will be passed as absl flags:
        group_name=group_name,
        task_name=f"{task_class}_{dataset}_{model}_{R}_{E}_{C}_{B}",
        model=model,
        dataset=dataset,
        R=R,
        E=E,
        C=C,
        B=B,
    )


def main(_):
    benchmark_name = FLAGS.benchmark_name
    assert benchmark_name in benchmarks
    run_benchmark(benchmark_name=benchmark_name)
