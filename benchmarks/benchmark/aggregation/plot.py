from typing import List, Optional, Tuple

import matplotlib
import numpy as np
from absl import flags
from matplotlib.colors import ListedColormap
from numpy import ndarray

from benchmarks.helpers import storage
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
    xlabel: str = "Epoch",
    ylabel: str = None,
    fname: Optional[str] = None,
    save: bool = True,
    show: bool = False,
    ylim_max: float = 1.0,
    xlim_max: float = 40.0,
    xticks_args: Optional[Tuple[List[int], List[str]]] = None,
    legend_loc: str = "lower right",
    vline: bool = False,
) -> str:
    """
    :param data: List of tuples where each represents a line in the plot
                 with tuple beeing (name, values, Optional[indices])

    :returns: For save=True returns absolut path to saved file otherwise None
    """
    assert fname is not None

    # if fname is an absolute path use fname directly otherwise assume
    # fname is filename and prepend output_dir
    fname_abspath = storage.fname_with_default_dir(fname, FLAGS.output_dir)

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

    if vline:
        plt.axvline(x=50.0)

    data.sort(key=lambda c: c[0])

    legend = []
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
        plt.savefig(fname=fname_abspath, format=FORMAT)
    if show:
        plt.show()
    plt.close()

    return fname_abspath


def plot_history_data(
    matrix: ndarray,
    file_name: str,
    title: Optional[str] = None,
    xlabel: str = "Training rounds",
    ylabel: str = "Participants",
    save: bool = True,
    show: bool = False,
) -> str:
    """Plot participant selection history.

    Plots or saves the participant selection history of an input matrix
    as 2D regular raster with training rounds vs participants.

    Args:
        matrix (~numpy.ndarray): Image data for a 2D regular raster.
        file_name (str): File name for storable plot.
        title (~typing.Optional[str]): Title of the plot.
        xlabel (str): Label for x-axis.
        ylabel (str): Label for y-axis.
        save (bool): If the plot should be stored as png.
        show (bool): If the plot should be shown.

    Returns:
        str: File name of the plot as absolute path.
    """

    file_name_abspath: str = storage.fname_with_default_dir(
        fname=file_name, dname=FLAGS.output_dir
    )

    # Creating figure with subplot
    _, ax = plt.subplots(figsize=(11, 9))

    # more information about colormaps: https://matplotlib.org/3.1.1/tutorials/colors/colormaps.html
    color_map_name: str = "YlGn"
    color_map: ListedColormap = prepare_colormap(name=color_map_name)

    im = ax.imshow(matrix, cmap=color_map, interpolation="nearest", aspect="auto")

    ax.figure.colorbar(im, ax=ax)

    # maxima for x and y axis
    x_max: int = matrix.shape[1]
    y_max: int = matrix.shape[0]

    # Major ticks
    ax.set_xticks(np.arange(0, x_max, 1))
    ax.set_yticks(np.arange(0, y_max, 5))

    # Labels for major ticks
    ax.set_xticklabels(np.arange(1, x_max, 1))
    ax.set_yticklabels(np.arange(0, y_max, 5))

    # Display only each 2nd x-axis tick label
    for x_tick_label in ax.xaxis.get_ticklabels()[::2]:
        x_tick_label.set_visible(False)

    # Minor ticks
    ax.set_xticks(np.arange(-0.5, x_max, 1), minor=True)
    ax.set_yticks(np.arange(-0.5, y_max, 1), minor=True)

    # turn off gridline visibility for x and y axis
    ax.tick_params(axis="both", which="minor", length=0)

    # Gridlines based on minor ticks
    ax.grid(which="minor", color="w", linestyle="-", linewidth=2)

    ax.set_xlabel(xlabel, fontsize=14)
    ax.set_ylabel(ylabel, fontsize=14)

    # Set plot title if present
    if title is not None:
        ax.set_title(title, fontsize=14)

    plt.tight_layout()

    # Saving and showing plot
    if save:
        plt.savefig(fname=file_name_abspath, format=FORMAT)
    if show:
        plt.show(block=True)
    plt.close()

    return file_name_abspath


def prepare_colormap(name: str) -> ListedColormap:
    """Creates a colormap with color grey as 0 value.

    Prepares a downscaled colormap (256 -> 40) from a colormap name and
    puts in color grey for value 0.

    Args:
        name (str): Name of the colormap to be scaled.

    Returns:
        ~matplotlib.colors.ListedColormap: Downscaled color map.
    """

    # downscaled colormap (256 -> 40)
    color_map: ListedColormap = matplotlib.cm.get_cmap(name, 40)
    map_scaled: ndarray = color_map(np.linspace(0, 1, 40))

    grey: ndarray = np.array([0, 0, 0, 0.1])
    # put in color grey as first entry
    map_scaled[0, :] = grey

    return ListedColormap(map_scaled)
