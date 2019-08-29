from absl import app, flags

from . import run

FLAGS = flags.FLAGS


"""
In this config the key in the dictionary will be the name of the benchmark
"""
benchmarks = {
    "fashion-mnist-100p-noniid-01cpp": {
        "dataset_name": "fashion-mnist-100p-noniid-01cpp"
    },
    "fashion-mnist-100p-noniid-02cpp": {
        "dataset_name": "fashion-mnist-100p-noniid-02cpp"
    },
    "fashion-mnist-100p-noniid-03cpp": {
        "dataset_name": "fashion-mnist-100p-noniid-03cpp"
    },
    "fashion-mnist-100p-noniid-04cpp": {
        "dataset_name": "fashion-mnist-100p-noniid-04cpp"
    },
    "fashion-mnist-100p-noniid-05cpp": {
        "dataset_name": "fashion-mnist-100p-noniid-05cpp"
    },
    "fashion-mnist-100p-noniid-06cpp": {
        "dataset_name": "fashion-mnist-100p-noniid-06cpp"
    },
    "fashion-mnist-100p-noniid-07cpp": {
        "dataset_name": "fashion-mnist-100p-noniid-07cpp"
    },
    "fashion-mnist-100p-noniid-08cpp": {
        "dataset_name": "fashion-mnist-100p-noniid-08cpp"
    },
    "fashion-mnist-100p-noniid-09cpp": {
        "dataset_name": "fashion-mnist-100p-noniid-09cpp"
    },
    "fashion-mnist-100p-iid-balanced": {
        "dataset_name": "fashion-mnist-100p-iid-balanced"
    },
    "integration_test": {
        "dataset_name": "fashion-mnist-100p-noniid-01cpp",
        "C": 0.02,  # two participants
        "E": 2,  # two epochs per round
        "rounds": 2,  # two rounds
    },
}


def main(_):
    benchmark_name = FLAGS.benchmark_name
    kwargs = benchmarks[benchmark_name]
    run.unitary_versus_federated(benchmark_name=benchmark_name, **kwargs)


if __name__ == "__main__":
    # Flags will be overriden by manually set flags as they will be parsed
    # again in the app.run invokation and overrides those set here
    FLAGS(["_", "--benchmark_name=fashion-mnist-100p-iid-balanced"])
    app.run(main=main)
