from absl import app

from autofl.benchmark import fl


def main():
    app.run(main=fl.main)


if __name__ == "__main__":
    main()
