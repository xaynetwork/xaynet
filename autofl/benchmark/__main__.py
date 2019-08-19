from absl import app, flags

from autofl.benchmark import bench_ea, bench_fl

FLAGS = flags.FLAGS


def main(_):
    if FLAGS.benchmark_type == "fl":
        if FLAGS.benchmark_name is None:
            raise Exception("flag benchmark_name is required for benchmark_type fl")
        bench_fl.main()
    elif FLAGS.benchmark_type == "ea":
        bench_ea.main()


if __name__ == "__main__":
    app.run(main=main)
