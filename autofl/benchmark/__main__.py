from absl import app, flags

from autofl.benchmark import bench_fl

# following: https://abseil.io/docs/cpp/guides/flags#flags-best-practices
# we will define our flags in this file
FLAGS = flags.FLAGS

flags.DEFINE_string(
    "benchmark_name",
    None,
    "Name of the benchmark to be run. Example: bench_fl.py"
    + " - other modules should use it in similar ways",
)

flags.mark_flag_as_required("benchmark_name")


def main():
    app.run(main=bench_fl.main)


if __name__ == "__main__":
    main()
