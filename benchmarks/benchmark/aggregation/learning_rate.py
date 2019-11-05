import os
from typing import List, Optional, Tuple

from absl import app, flags, logging

from xain.helpers import storage
from xain.types import PlotValues

from .plot import plot
from .results import GroupResult, TaskResult

FLAGS = flags.FLAGS


def read_task_values(task_result: TaskResult) -> Tuple[str, Optional[List[float]]]:
    """Reads unitary and federated accuracy from results.json

    Args:
        fname (str): path to results.json file containing required fields

    Returns
        class, label, final_accuracy (str, str, float): e.g. ("VisionTask", "cpp01", 0.92)
    """
    return (task_result.get_label(), task_result.get_learning_rates())


def read_all_task_values(group_dir: str) -> List[Tuple[str, List[float]]]:
    """
    Reads results directory for given group_dir and returns a list of
    tuples with label and list of learning rates

    Args:
        group_dir (str): path to directory to be read

    """
    task_results = GroupResult(group_dir).get_results()
    # Reatur accuracies from each file and return list of values in tuples
    all_tasks = [read_task_values(task_result) for task_result in task_results]

    federated_tasks = [
        (label, learning_rates)
        for label, learning_rates in all_tasks
        if learning_rates is not None
    ]

    return federated_tasks


def prepare_aggregation_data(group_name: str) -> List[PlotValues]:
    """Constructs and returns learning rate curves

    Args:
        group_name (str): group name for which to construct the curves

    Returns:
        A list of `PlotValues`.
    """
    group_dir = os.path.join(FLAGS.results_dir, group_name)
    # List of tuples (benchmark_name, unitary_accuracy, federated_accuracy)
    labels_and_lrs = read_all_task_values(group_dir=group_dir)

    assert labels_and_lrs, "No values for group found"

    return [
        (label, lrs, [i for i in range(1, len(lrs) + 1, 1)])
        for label, lrs in labels_and_lrs
    ]


def aggregate() -> str:
    """Plots learning rate for federated tasks in a group

    :param data: List of tuples which represent (name, values, indices)
    :param fname: Filename of plot

    :returns: Absolut path to saved plot
    """
    group_name = FLAGS.group_name
    dname = storage.create_output_subdir(group_name)
    fname = storage.fname_with_default_dir("plot_learning_rates.png", dname)

    data = prepare_aggregation_data(group_name)

    ylim_max: float = 0
    xlim_max = 0

    for _, lrs, ylabel in data:
        if ylabel is not None:
            xlim_max = max(ylabel + [xlim_max])

        for lr in lrs:
            ylim_max = max(lr, ylim_max)

    ylim_max *= 1.1
    xlim_max += 1

    assert data, "Expecting a list with at least one item"

    fpath = plot(
        data,
        title="Optimizer learning rates for federated training tasks",
        xlabel="round",
        ylabel="learning rate",
        fname=fname,
        ylim_max=ylim_max,
        xlim_max=xlim_max,
        legend_loc="upper right",
    )

    logging.info(f"Data plotted and saved in {fpath}")

    return fpath


def app_run_aggregate():
    flags.mark_flag_as_required("group_name")
    app.run(main=lambda _: aggregate())
