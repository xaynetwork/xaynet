from absl import app, flags

from benchmarks.benchmark import benchmark

FLAGS = flags.FLAGS


def main():
    flags.mark_flag_as_required("benchmark_name")
    app.run(main=benchmark.main)


if __name__ == "__main__":
    main()
