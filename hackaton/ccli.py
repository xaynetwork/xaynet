import argparse
import numpy as np

from xain_fl.grpc.coordinator import Coordinator, serve


def main() -> None:
    parser = argparse.ArgumentParser(description="Coordinator CLI")

    parser.add_argument(
        "-f", dest="file", required=True, help="Path to numpy files",
    )
    parser.add_argument(
        "-r",
        dest="num_rounds",
        default=2,
        type=int,
        choices=range(1, 10),
        help="Number of global rounds the model is going to be trained for.",
    )
    parser.add_argument(
        "-p",
        dest="num_participants",
        default=1,
        type=int,
        choices=range(1, 4),
        help="Number of participants.",
    )
    parser.add_argument(
        "-c",
        dest="fraction",
        default=1,
        help="Fraction of total clients that participate in a training round.",
    )
    parser.add_argument(
        "-e",
        dest="num_epochs",
        default=2,
        choices=range(1, 10),
        help="Fraction of total clients that participate in a training round.",
    )

    parameters = parser.parse_args()

    save_array = np.load(parameters.file, allow_pickle=True)
    theta = list(save_array)

    coordinator = Coordinator(
        num_rounds=parameters.num_rounds,
        required_participants=parameters.num_participants * parameters.fraction,
        theta=theta,
        epochs=parameters.num_epochs,
    )

    serve(coordinator)


if __name__ == "__main__":
    main()
