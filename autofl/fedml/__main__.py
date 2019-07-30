from absl import app

from autofl.fedml import fedml


def main():
    app.run(main=fedml.federated_learning)


if __name__ == "__main__":
    main()
