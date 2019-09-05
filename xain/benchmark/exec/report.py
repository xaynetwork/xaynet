import os
from typing import Dict, List, Optional, Tuple

import matplotlib
import numpy as np
from absl import app, flags, logging

from xain.helpers import storage

matplotlib.use("AGG")

# To avoid issues with tkinter we need to set the renderer
# for matplotlib before importing pyplot
# As isort would move this line under the "import matplotlib"
# We need to skip isort explicitly
# pylint: disable-msg=wrong-import-position, wrong-import-order
import matplotlib.pyplot as plt  # isort:skip


FORMAT: str = "png"

FLAGS = flags.FLAGS


def get_task_class(data: Dict) -> str:
    return data["task_name"].split("_")[0]


def get_task_label(data: Dict) -> str:
    return data["dataset"].split("-")[-1]


def get_task_accuracy(data: Dict) -> float:
    return data["acc"]


def read_task_values(fname: str) -> Tuple[str, str, float]:
    """Reads unitary and federated accuracy from results.json

    Args:
        fname (str): path to results.json file containing required fields

    Returns
        task_name, task_label, accuracy (str, str, float)
    """
    data = storage.read_json(fname)
    return (get_task_class(data), get_task_label(data), get_task_accuracy(data))


def read_all_task_values(group_dir: str) -> List[Tuple[str, str, float]]:
    """
    Reads results directory for given group id and
    extracts values from results.json files

    :param filter_substring: has to be part of the dir name in results directory

    :returns: List of tuples (benchmark_name, unitary_accuracy, federated_accuracy)
    """
    assert os.path.isdir(group_dir)

    # get list of all directories which contain given substring
    json_files = [
        fname
        for fname in storage.listdir_recursive(group_dir, relpath=False)
        if fname.endswith("results.json")
    ]

    if not json_files:
        raise Exception(f"No values results found in group_dir: {group_dir}")

    # Read accuracies from each file and return list of values in tuples
    return [read_task_values(fname) for fname in json_files]


def group_values_by_class(
    values: List[Tuple[str, str, float]]
) -> Dict[str, List[Tuple[str, float]]]:
    # Get unique task classes
    task_classes = np.unique([v[0] for v in values])

    # Group values by task_class
    grouped_values: Dict[str, List[Tuple[str, float]]] = {
        task_class: [] for task_class in task_classes
    }

    for task_class in task_classes:
        filtered_values = [v for v in values if v[0] == task_class]
        for value in filtered_values:
            (_, label, acc) = value
            grouped_values[task_class].append((label, acc))

    return grouped_values


PlotValues = Tuple[str, List[float], Optional[List[int]]]
XticksLocations = List[int]
XticksLabels = List[str]


def prepare_comparison_data(
    group_name: str
) -> Tuple[List[PlotValues], Optional[Tuple[XticksLocations, XticksLabels]]]:
    """Constructs and returns curves and xticks_args

    Args:
        group_name (str): group name for which to construct the curves

    Returns:
        (
            [
                ("unitary", unitary_accuracies, indices),
                ("federated", federated_accuracies, indices)
            ],
            (xticks_locations, xticks_labels))
        )
    """
    group_dir = os.path.join(FLAGS.results_dir, group_name)
    # List of tuples (benchmark_name, unitary_accuracy, federated_accuracy)
    values = read_all_task_values(group_dir=group_dir)
    values = sorted(values, key=lambda v: v[1], reverse=True)  # sort by

    assert values, "No values for group found"

    grouped_values = group_values_by_class(values)
    task_classes = [k for k in grouped_values]
    indices = list(range(1, len(grouped_values[task_classes[0]]) + 1))
    labels = [label for label, _ in grouped_values[task_classes[0]]]

    data: List[PlotValues] = []

    for task_class in grouped_values:
        task_class_values = [acc for _, acc in grouped_values[task_class]]
        plot_values = (task_class, task_class_values, indices)
        data.append(plot_values)

    return (data, (indices, labels))


def plot_final_task_accuracies() -> str:
    """Plots IID and Non-IID dataset performance comparision

    :param data: List of tuples which represent (name, values, indices)
    :param fname: Filename of plot

    :returns: Absolut path to saved plot
    """
    group_name = FLAGS.group_name
    fname = f"plot_{group_name}.png"

    (data, xticks_args) = prepare_comparison_data(group_name)

    assert len(data) == 2, "Expecting a list of two curves"

    fpath = _plot(
        data,
        title="Max achieved accuracy for unitary and federated learning",
        xlabel="partitioning grade",
        ylabel="accuracy",
        fname=fname,
        save=True,
        show=False,
        ylim_max=1.0,
        xlim_max=12,
        xticks_args=xticks_args,
        legend_loc="upper right",
    )

    logging.info(f"Data plotted and saved in {fname}")

    return fpath


def plot_accuracies(
    data: List[Tuple[str, List[float], Optional[List[int]]]], fname="plot.png"
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


def app_run_plot_final_task_accuracies():
    flags.mark_flag_as_required("group_name")
    app.run(main=lambda _: plot_final_task_accuracies())
