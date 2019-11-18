"""Used as entry_point for console_script `train_remote` in `setup.py`.

Expects the following flags:

    - benchmark_name

Example:
    train_remote --benchmark_name=flul-fashion-mnist-100p-iid-balanced --group_name=GROUP_NAME
"""
from absl import app, flags

from benchmarks.benchmark import benchmark

FLAGS = flags.FLAGS


def main():
    flags.mark_flag_as_required("benchmark_name")
    app.run(main=benchmark.main)


if __name__ == "__main__":
    main()
