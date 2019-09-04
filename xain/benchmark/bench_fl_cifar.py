from absl import app, flags

from .exec import run

FLAGS = flags.FLAGS


"""
In this config the key in the dictionary will be the name of the benchmark
"""
benchmarks = {
    "cifar-10-100p-noniid-01cpp": {"dataset_name": "cifar-10-100p-noniid-01cpp"},
    "cifar-10-100p-noniid-02cpp": {"dataset_name": "cifar-10-100p-noniid-02cpp"},
    "cifar-10-100p-noniid-03cpp": {"dataset_name": "cifar-10-100p-noniid-03cpp"},
    "cifar-10-100p-noniid-04cpp": {"dataset_name": "cifar-10-100p-noniid-04cpp"},
    "cifar-10-100p-noniid-05cpp": {"dataset_name": "cifar-10-100p-noniid-05cpp"},
    "cifar-10-100p-noniid-06cpp": {"dataset_name": "cifar-10-100p-noniid-06cpp"},
    "cifar-10-100p-noniid-07cpp": {"dataset_name": "cifar-10-100p-noniid-07cpp"},
    "cifar-10-100p-noniid-08cpp": {"dataset_name": "cifar-10-100p-noniid-08cpp"},
    "cifar-10-100p-noniid-09cpp": {"dataset_name": "cifar-10-100p-noniid-09cpp"},
    "cifar-10-100p-iid-balanced": {"dataset_name": "cifar-10-100p-iid-balanced"},
    "integration_test": {
        "dataset_name": "cifar-10-100p-noniid-01cpp",
        "R": 2,  # two rounds
        "E": 2,  # two epochs per round
        "C": 0.02,  # two participants
    },
}


def main(_):
    benchmark_name = FLAGS.benchmark_name
    kwargs = benchmarks[benchmark_name]
    run.unitary_versus_federated(
        benchmark_name=benchmark_name, model_name="resnet20", **kwargs
    )


if __name__ == "__main__":
    # Flags will be overriden by manually set flags as they will be parsed
    # again in the app.run invokation and overrides those set here
    FLAGS(["_", "--benchmark_name=cifar-10-100p-iid-balanced"])
    app.run(main=main)
