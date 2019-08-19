import json
import os
from typing import Dict, List, Optional, Tuple

import matplotlib.pyplot as plt
from absl import flags

FORMAT: str = "png"

FLAGS = flags.FLAGS


def get_abspath(fname: str) -> str:
    if os.path.isabs(fname):
        return fname

    return os.path.join(FLAGS.output_dir, fname)


def write_json(results: Dict, fname="benchmark_results.json"):
    fname = get_abspath(fname)
    with open(fname, "w") as outfile:
        json.dump(results, outfile, indent=2, sort_keys=True)


def plot_idd_cpp_comparision(
    data: List[Tuple[str, List[float], Optional[List[int]]]], fname="benchmark_plot.png"
) -> str:
    """
    Plots IDD and Ncpp dataset performance comparision

    :param data: List of tuples which represent (name, values, indices)
    :param fname: Filename of plot

    :returns: Absolut path to saved plot
    """
    assert len(data) == 2, "Expecting a list of two curves"

    xticks_locations = list(range(1, 12, 1))
    xticks_labels = ["IDD"] + [str(n) for n in range(10, 0, -1)]

    return _plot(
        data,
        title="Max achieved accuracy for unitary and federated learning",
        xlabel="partitioning grade",
        ylabel="accuracy",
        fname=fname,
        save=True,
        show=False,
        ylim_max=1.0,
        xlim_max=12,
        xticks_args=(xticks_locations, xticks_labels),
    )


def plot_accuracies(
    data: List[Tuple[str, List[float], Optional[List[int]]]], fname="benchmark_plot.png"
) -> str:
    """
    :param data: List of tuples where each represents a line in the plot
                 with tuple beeing (name, values, indices)
    :param fname: Filename of plot

    :returns: Absolut path to saved plot
    """
    # Take highest length of values list as xlim_max
    xlim_max = max([len(values) for _, values, _ in data])

    return _plot(
        data,
        title="Validation set accuracy for unitary and federated learning",
        ylabel="accuracy",
        fname=fname,
        save=True,
        show=False,
        ylim_max=1.0,
        xlim_max=xlim_max,
    )


def _plot(
    data: List[Tuple[str, List[float], Optional[List[int]]]],
    title: Optional[str] = None,
    xlabel: str = "epoch",
    ylabel: str = None,
    fname: Optional[str] = None,
    save: bool = True,
    show: bool = False,
    ylim_max: float = 1.0,
    xlim_max: float = 40.0,
    xticks_args: Optional[Tuple[List[int], List[str]]] = None,
) -> str:
    """
    :param data: List of tuples where each represents a line in the plot
                 with tuple beeing (name, values, indices)

    :returns: For save=True returns absolut path to saved file otherwise None
    """
    assert fname is not None

    fname_abspath = get_abspath(fname)

    plt.figure()
    plt.ylim(0.0, ylim_max)
    plt.xlim(0.0, xlim_max)

    if xticks_args is not None:
        plt.xticks(*xticks_args)

    if title is not None:
        plt.title(title)

    plt.xlabel(xlabel)
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
        # if fname is an absolute path use fname directly otherwise assume
        # fname is filename and prepend output_dir
        plt.savefig(fname=fname_abspath, format=FORMAT)
    if show:
        plt.show()
    plt.close()

    return fname_abspath
