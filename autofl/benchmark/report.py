import json
import os
from typing import Dict, List, Optional, Tuple

import matplotlib.pyplot as plt

FORMAT: str = "png"


def write_json(results: Dict, fname="benchmark_results.json", plot_dir="output"):
    fname = os.path.join(plot_dir, fname)
    with open(fname, "w") as outfile:
        json.dump(results, outfile, indent=2, sort_keys=True)


def plot_accuracies(
    data: List[Tuple[str, List[float], Optional[List[int]]]],
    fname="benchmark_plot.png",
    plot_dir="output",
):
    """
    :param data: List of tuples where each represents a line in the plot
                 with tuple beeing (name, values, indices)
    """
    # Take highest length of values list as xlim_max
    xlim_max = max([len(values) for _, values, _ in data])

    plot(
        data,
        title="Validation set accuracy for unitary and federated learning",
        ylabel="accuracy",
        plotdir=plot_dir,
        fname=fname,
        save=True,
        show=False,
        ylim_max=1.0,
        xlim_max=xlim_max,
    )


def plot(
    data: List[Tuple[str, List[float], Optional[List[int]]]],
    title: Optional[str] = None,
    ylabel: str = None,
    plotdir: Optional[str] = None,
    fname: Optional[str] = None,
    save: bool = True,
    show: bool = False,
    ylim_max: float = 1.0,
    xlim_max: float = 40.0,
) -> None:
    """
    :param data: List of tuples where each represents a line in the plot
                 with tuple beeing (name, values, indices)
    """
    assert fname is not None

    plt.figure()
    plt.ylim(0.0, ylim_max)
    plt.xlim(0.0, xlim_max)

    if title is not None:
        plt.title(title)

    plt.xlabel("epoch")
    plt.ylabel(ylabel)

    legend = []

    for name, values, indices in data:
        legend.append(name)

        if indices is None:
            # x values are optional and default to range(len(values))
            plt.plot(values)
        else:
            assert len(values) == len(indices)
            plt.plot(indices, values)

    plt.legend(legend, loc="lower right")

    if save:
        fname = fname if plotdir is None else plotdir + "/" + fname
        plt.savefig(fname=fname, format=FORMAT)
    if show:
        plt.show()
    plt.close()
