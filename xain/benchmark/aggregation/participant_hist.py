"""Functions for getting participant selection history"""

import os
from typing import List, Optional, Tuple

import matplotlib.pyplot as plt
import numpy as np
from absl import app, flags, logging
from matplotlib import cm
from matplotlib.colors import ListedColormap
from numpy import ndarray

from xain.benchmark.aggregation.results import GroupResult, TaskResult
from xain.helpers.storage import get_abspath
from xain.types import Metrics

FORMAT = "png"

FLAGS = flags.FLAGS


def get_participant_history() -> List[str]:
    """Plot participant selection histories for group name flag.

    For each task result in the group name flag extract the task metrics (number of
    participants, task label, hist metrics), transform them into heatmap data as participant
    indices x training rounds and plot/save them as participant selection history.

    Returns:
        ~typing.List[str]: File paths for saved plots.
    """

    group_name: str = FLAGS.group_name
    file_name: str = "plot_participant_history_{}.png".format(group_name)
    file_paths: List[str] = list()

    # getting history metrics data from results.json
    hist_metrics: List[Tuple[int, List[List[Metrics]]]] = get_hist_metrics(
        group_name=group_name
    )

    #
    matrices: List[ndarray] = list(map(heatmap_data, hist_metrics))

    for matrix in matrices:
        file_path: str = plot_history_data(
            matrix=matrix,
            title="Participant Selection History",
            file_name=file_name,
            save=False,
            show=True,
        )
        file_paths.append(file_path)

    logging.info(f"Data plotted and saved in {file_path}")

    return file_paths


def get_hist_metrics(group_name: str) -> List[Tuple[int, str, List[List[Metrics]]]]:
    """Get the task metrics for a group name.

    Extract number of participants, task label and hist metrics for each federated task
    metric in a group name and put them into a list.

    Args:
        group_name (str): Group name for which metrics should be extracted.

    Returns:
        List[Tuple[int, str, List[List[Metrics]]]]: List of metrics for each task.
    """

    group_dir: str = os.path.join(FLAGS.results_dir, group_name)
    task_results: List[TaskResult] = GroupResult(group_dir=group_dir).get_results()

    # Extract metrics for each federated task result in group result
    metrics: List[Tuple[int, str, List[List[Metrics]]]] = [
        read_task_metrics(task_result=task_result)
        for task_result in task_results
        if task_result.is_unitary() is False
    ]

    return metrics


def heatmap_data(metric: Tuple[int, str, List[List[Metrics]]]) -> Tuple[str, ndarray]:
    """Heatmap data for a given task metric.

    Creates the heatmap data as participant x train rounds numpy matrix for a given task metric
    and sets those matrix values to 1, that indices correspond to participant indices and train
    rounds in the given task history metrics.

    Args:
        metric (Tuple[int, List[List[Metrics]]]): Task metric consisting of number of participants
            and hist metrics for a task.

    Returns:
        ~typing.Tuple[str, numpy.ndarray]: Task label and participant x train rounds matrix with
            value 1 for each participant indice
    """

    num_participants, task_label, hist_metrics = metric
    train_rounds: int = len(hist_metrics)

    # Create heatmap matrix with participants as rows and train rounds as columns
    heat_map: ndarray = np.zeros((num_participants, train_rounds))

    # Collect participants indices for each training round
    rows: ndarray = np.asarray(hist_metrics, dtype=object)[:, :, 0].astype(int)
    # Array of linspace training rounds, each wrapped into an array
    columns: ndarray = np.split(np.arange(train_rounds), train_rounds)
    heat_map[rows, columns] = 1

    return (task_label, heat_map)


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

    file_name_abspath: str = get_abspath(fname=file_name, dname=FLAGS.output_dir)

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


def read_task_metrics(task_result: TaskResult) -> Tuple[int, str, List[List[Metrics]]]:
    """Get number of participants, task label and history metrics for a task result.

    Args:
        task_result (TaskResult): Results data for a task.

    Returns:
        Tuple[int, str, List[List[Metrics]]]: Number of participants, task label and hist
            metrics in a tuple.
    """

    return (
        task_result.get_num_participants(),
        task_result.get_label(),
        task_result.get_hist_metrics(),
    )


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
    color_map: ListedColormap = cm.get_cmap(name, 40)
    map_scaled: ndarray = color_map(np.linspace(0, 1, 40))

    grey: ndarray = np.array([0, 0, 0, 0.1])
    # put in color grey as first entry
    map_scaled[0, :] = grey

    return ListedColormap(map_scaled)


def app_run_participant():
    flags.mark_flag_as_required("group_name")
    app.run(main=lambda _: get_participant_history())
