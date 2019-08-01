from absl import app

from autofl.benchmark import bench_fl


def main():
    app.run(main=bench_fl.main)


if __name__ == "__main__":
    main()
