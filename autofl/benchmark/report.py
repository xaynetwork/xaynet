from typing import Optional

import matplotlib.pyplot as plt

FORMAT: str = "png"


def plot_accuracies(history_ul, history_fl, plot_dir=None):
    fname = "fl_vs_ul.png"
    plot_curves(
        curves=[history_ul.history["val_acc"], history_fl.history["val_acc"]],
        legend=["UL", "FL"],
        title="Validation set accuracy for unitary and federated learning",
        ylabel="accuracy",
        plotdir=plot_dir,
        fname=fname,
        save=True,
        show=False,
        ylim_max=1.0,
    )


# pylint: disable-msg=too-many-arguments
def plot_curves(
    curves,
    legend,
    title: Optional[str] = None,
    ylabel: str = None,
    plotdir: Optional[str] = None,
    fname: Optional[str] = None,
    save: bool = True,
    show: bool = False,
    ylim_max: float = 1.0,
) -> None:
    assert len(curves) == len(legend)
    assert fname is not None
    plt.figure()
    plt.ylim(0.0, ylim_max)
    plt.xlim(0.0, 40.0)
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
