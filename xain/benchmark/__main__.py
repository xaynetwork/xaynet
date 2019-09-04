from absl import app, flags

from . import benchmark

FLAGS = flags.FLAGS


def main():
    FLAGS(["_", "--benchmark_name=flul-fashion-mnist-100p-iid-balanced"])
    app.run(main=benchmark.main)


if __name__ == "__main__":
    main()
