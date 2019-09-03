from absl import app, flags

from xain.benchmark import bench_ea, bench_fl, bench_fl_cifar

FLAGS = flags.FLAGS
benchmark_names_fl = [n for n in bench_fl.benchmarks]
benchmark_names_fl_cifar = [n for n in bench_fl_cifar.benchmarks]


def main(_):
    if FLAGS.benchmark_type == "fl":
        bench_fl.main(_)
    if FLAGS.benchmark_type == "fl_cifar":
        bench_fl_cifar.main(_)
    elif FLAGS.benchmark_type == "ea":
        bench_ea.main(_)


def main_wrapper():
    flags.register_validator(
        "benchmark_name",
        lambda benchmark_name: (
            FLAGS.benchmark_type == "fl" and benchmark_name in benchmark_names_fl
        )
        or (
            FLAGS.benchmark_type == "fl_cifar"
            and benchmark_name in benchmark_names_fl_cifar
        )
        or FLAGS.benchmark_type == "ea",
        message="--benchmark_name must be set for benchmark_type=fl. Valid values:"
        + ", ".join(benchmark_names_fl),
    )

    # This wrapper is needed for console_scripts as
    # they cant directly execute a module
    app.run(main=main)


if __name__ == "__main__":
    main_wrapper()
