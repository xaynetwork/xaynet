from absl import app, flags

from . import benchmark

FLAGS = flags.FLAGS


def main():
    # mark_flag_as_required only works if no default in __init__.py is set
    # flags.mark_flag_as_required("benchmark_name")
    app.run(main=benchmark.main)


if __name__ == "__main__":
    main()
