from absl import app, flags

from autofl.benchmark import bench_ea, bench_fl

# following: https://abseil.io/docs/cpp/guides/flags#flags-best-practices
# we will define our flags in this file
FLAGS = flags.FLAGS

flags.DEFINE_enum("benchmark_type", "fl", ["fl", "ea"], "Type of benchmark to run")

flags.DEFINE_string(
    "benchmark_name",
    None,
    "Name of the benchmark to be run. Example: bench_fl.py"
    + " - other modules should use it in similar ways",
)


def main(_):
    if FLAGS.benchmark_type == "fl":
        if FLAGS.benchmark_name is None:
            raise Exception("flag benchmark_name is required for benchmark_type fl")
        bench_fl.main()
    elif FLAGS.benchmark_type == "ea":
        bench_ea.main()


if __name__ == "__main__":
    app.run(main=main)
