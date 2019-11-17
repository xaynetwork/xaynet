import os
from typing import Dict, List, Optional, Tuple

from absl import app, flags, logging

from benchmarks.helpers import storage
from xain.types import PlotValues, XticksLabels, XticksLocations

from .plot import plot
from .results import GroupResult, TaskResult

FLAGS = flags.FLAGS


def read_task_values(task_result: TaskResult) -> Tuple[bool, str, float]:
    """Reads unitary and federated accuracy from results.json

    Args:
        fname (str): path to results.json file containing required fields

    Returns
        class, label, final_accuracy (str, str, float): e.g. ("VisionTask", "cpp01", 0.92)
    """
    return (
        task_result.is_unitary(),
        task_result.get_label(),
        task_result.get_final_accuracy(),
    )


def read_all_task_values(group_dir: str) -> List[Tuple[bool, str, float]]:
    """
    Reads results directory for given group id and
    extracts values from results.json files

    Args:
        group_dir (str): path to directory to be read

    """
    task_results = GroupResult(group_dir).get_results()
    # Read accuracies from each file and return list of values in tuples
    return [read_task_values(task_result) for task_result in task_results]


def group_values_by_class(
    values: List[Tuple[bool, str, float]]
) -> Dict[str, List[Tuple[str, float]]]:
    # Group values by task_class
    unitary_values = [v for v in values if v[0]]
    federated_values = [v for v in values if not v[0]]

    grouped_values = {
        "Unitary": [(label, acc) for _, label, acc in unitary_values],
        "Federated": [(label, acc) for _, label, acc in federated_values],
    }

    return grouped_values


def prepare_aggregation_data(
    group_name: str
) -> Tuple[List[PlotValues], Optional[Tuple[XticksLocations, XticksLabels]]]:
    """Constructs and returns curves and xticks_args.

    Args:
        group_name (str): group name for which to construct the curves

    Returns:
        A tuple containing a list of `PlotValues` and a list of tuples
        containing (`XticksLocations`, `XticksLabels`)

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


def aggregate() -> str:
    """Plots IID and Non-IID dataset performance comparision

    :param data: List of tuples which represent (name, values, indices)
    :param fname: Filename of plot

    :returns: Absolut path to saved plot
    """
    group_name = FLAGS.group_name
    dname = storage.create_output_subdir(group_name)
    fname = storage.fname_with_default_dir("plot_final_task_accuracies.png", dname)

    (data, xticks_args) = prepare_aggregation_data(group_name)

    assert len(data) == 2, "Expecting a list of two curves"

    fpath = plot(
        data,
        title="",  # TODO
        xlabel="IID / Non-IID",
        ylabel="Accuracy",
        fname=fname,
        ylim_max=1.0,
        xlim_max=12,
        xticks_args=xticks_args,
        legend_loc="upper right",
    )

    logging.info(f"Data plotted and saved in {fpath}")

    return fpath
