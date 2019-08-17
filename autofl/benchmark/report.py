import json
from typing import Dict, List, Optional, Tuple

import matplotlib.pyplot as plt

FORMAT: str = "png"


def write_json(results: Dict, fname="benchmark_results.json", plot_dir="output"):
    fname = plot_dir + "/" + fname
    with open(fname, "w") as outfile:
        json.dump(results, outfile, indent=2, sort_keys=True)


def plot_accuracies(
    history_ul: Dict[str, List[float]],
    history_fl: Dict[str, List[float]],
    fname="benchmark_plot.png",
    plot_dir="output",
):
    xlim_max = len(history_ul["val_acc"])
    plot_curves(
        curves=[history_ul["val_acc"], history_fl["val_acc"]],
        legend=["UL", "FL"],
        title="Validation set accuracy for unitary and federated learning",
        ylabel="accuracy",
        plotdir=plot_dir,
        fname=fname,
        save=True,
        show=False,
        ylim_max=1.0,
        xlim_max=xlim_max,
    )


# pylint: disable-msg=too-many-arguments
def plot_curves(
    curves: List[List[float]],
    legend: List[str],
    title: Optional[str] = None,
    ylabel: str = None,
    plotdir: Optional[str] = None,
    fname: Optional[str] = None,
    save: bool = True,
    show: bool = False,
    ylim_max: float = 1.0,
    xlim_max: float = 40.0,
) -> None:
    assert len(curves) == len(legend)
    assert fname is not None
    plt.figure()
    plt.ylim(0.0, ylim_max)
    plt.xlim(0.0, xlim_max)
    for c in curves:
        plt.plot(c)
    if title is not None:
        plt.title(title)
    plt.ylabel(ylabel)
    plt.xlabel("epoch")
    plt.legend(legend, loc="upper left")
    if save:
        fname = fname if plotdir is None else plotdir + "/" + fname
        plt.savefig(fname=fname, format=FORMAT)
    if show:
        plt.show()
    plt.close()


def plot_accs(
    data: List[Tuple[str, List[float], Optional[List[int]]]],
    fname="benchmark_plot.png",
    plot_dir="output",
):
    xlim_max = max([len(vs) for _, vs, _ in data])

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
    assert fname is not None

    plt.figure()
    plt.ylim(0.0, ylim_max)
    plt.xlim(0.0, xlim_max)

    legend = []
    for name, values, indices in data:
        legend.append(name)
        if indices is None:
            plt.plot(values)
        else:
            assert len(values) == len(indices)
            plt.plot(indices, values)
    plt.legend(legend, loc="lower right")

    if title is not None:
        plt.title(title)
    plt.ylabel(ylabel)
    plt.xlabel("epoch")
    if save:
        fname = fname if plotdir is None else plotdir + "/" + fname
        plt.savefig(fname=fname, format=FORMAT)
    if show:
        plt.show()
    plt.close()
