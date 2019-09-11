from typing import List, Optional, Tuple

import matplotlib
from absl import flags

from xain.helpers import storage
from xain.types import PlotValues

matplotlib.use("AGG")

# To avoid issues with tkinter we need to set the renderer
# for matplotlib before importing pyplot
# As isort would move this line under the "import matplotlib"
# We need to skip isort explicitly
# pylint: disable-msg=wrong-import-position, wrong-import-order
import matplotlib.pyplot as plt  # isort:skip

FORMAT = "png"

FLAGS = flags.FLAGS


def plot(
    data: List[PlotValues],
    title: Optional[str] = None,
    xlabel: str = "epoch",
    ylabel: str = None,
    fname: Optional[str] = None,
    save: bool = True,
    show: bool = False,
    ylim_max: float = 1.0,
    xlim_max: float = 40.0,
    xticks_args: Optional[Tuple[List[int], List[str]]] = None,
    legend_loc: str = "lower right",
) -> str:
    """
    :param data: List of tuples where each represents a line in the plot
                 with tuple beeing (name, values, indices)

    :returns: For save=True returns absolut path to saved file otherwise None
    """
    assert fname is not None

    fname_abspath = storage.get_abspath(fname, FLAGS.output_dir)

    plt.figure()
    plt.ylim(0.0, ylim_max)
    plt.xlim(0.0, xlim_max)

    if xticks_args is not None:
        xticks_locations, xticks_labels = xticks_args
        # if any label has length > 3 rotate labels by 90 degrees
        rot = 90 if any([len(l) > 3 for l in xticks_labels]) else 0
        plt.xticks(xticks_locations, xticks_labels, rotation=rot)

    if title is not None:
        plt.title(title)

    plt.xlabel(xlabel)
    plt.ylabel(ylabel)

    legend = []

    data.sort(key=lambda c: c[0])

    for name, values, indices in data:
        legend.append(name)

        if indices is None:
            # x values are optional and default to range(len(values))
            plt.plot(values)
        else:
            assert len(values) == len(indices)
            plt.plot(indices, values)

    plt.legend(legend, loc=legend_loc)

    # https://matplotlib.org/users/tight_layout_guide.html
    plt.tight_layout()

    if save:
        # if fname is an absolute path use fname directly otherwise assume
        # fname is filename and prepend output_dir
        plt.savefig(fname=fname_abspath, format=FORMAT)
    if show:
        plt.show()
    plt.close()

    return fname_abspath
