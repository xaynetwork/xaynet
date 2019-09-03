from typing import Callable, Dict, List

from absl import app, flags, logging

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
    #
    # ##############################
    # Class Partitioning
    # ##############################
    #
    "cpp-fashion-mnist-100p-iid-balanced": Benchmark(
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
}


def run_benchmark(benchmark_name: str):
    logging.info(f"Starting benchmark {benchmark_name}")
    benchmark = benchmarks[benchmark_name]
    aggregation_name = benchmark.aggregation_name
    # TODO run tasks in parallel
    for task in benchmark.tasks:
        model_name = task.model_name
        dataset_name = task.dataset_name
        run_task(model_name, dataset_name, R=task.R, E=task.E, C=task.C, B=task.B)
    # TODO wait for completion
    # Aggregate results
    aggregation_fn = aggregations[aggregation_name]
    aggregation_fn()


def run_task(model: str, dataset: str, R: int, E: int, C: float, B: int):
    logging.info(f"Run task: {model}, {dataset}, {R}, {E}, {C}, {B}")


def main(_):
    benchmark_name = FLAGS.benchmark_name
    assert benchmark_name in benchmarks.keys()
    run_benchmark(benchmark_name=benchmark_name)


if __name__ == "__main__":
    FLAGS(["_", "--benchmark_name=flul-fashion-mnist-100p-iid-balanced"])
    app.run(main=main)
