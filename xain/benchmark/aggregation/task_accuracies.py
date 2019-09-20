import os
from typing import List, Tuple

from absl import app, flags, logging

from xain.helpers import storage
from xain.types import PlotValues

from .plot import plot
from .results import GroupResult, TaskResult

FLAGS = flags.FLAGS


def read_task_values(task_result: TaskResult) -> Tuple[bool, str, List[float], int]:
    """Reads unitary and federated accuracy from results.json

    Args:
        fname (str): path to results.json file containing required fields

    Returns:
        task_label, accuracies, epochs (str, List[float], int): e.g. ("VisionTask", [0.12, 0.33], 5)
    """
    return (
        task_result.is_unitary(),
        task_result.get_label(),
        task_result.get_accuracies(),
        task_result.get_E(),
    )


def read_all_task_values(group_dir: str) -> List[Tuple[bool, str, List[float], int]]:
    """
    Reads results directory for given group id and
    extracts values from results.json files

    Args:
        group_dir: path to directory to be read
    """
    task_results = GroupResult(group_dir).get_results()
    # Read accuracies from each file and return list of values in tuples
    return [read_task_values(task_result) for task_result in task_results]


def build_plot_values(values: Tuple[bool, str, List[float], int]) -> PlotValues:
    """Returns PlotValues with appropriate indices based on task class (Unitary or Federated)"""
    is_unitary, task_label, task_accuracies, E = values

    if is_unitary:
        indices = [i for i in range(1, len(task_accuracies) + 1, 1)]
    else:
        indices = [i for i in range(E, len(task_accuracies) * E + 1, E)]

    return (task_label, task_accuracies, indices)


def prepare_aggregation_data(group_name: str) -> List[PlotValues]:
    """Constructs and returns curves and xticks_args

    Args:
        group_name (str): group name for which to construct the curves

    Returns:
        ([
            ("unitary", unitary_accuracies, indices),
            ("federated", federated_accuracies, indices)
        ])
    """
    group_dir = os.path.join(FLAGS.results_dir, group_name)
    # List of tuples (benchmark_name, unitary_accuracy, federated_accuracy)
    values = read_all_task_values(group_dir=group_dir)

    assert values, "No values for group found"

    data: List[PlotValues] = list(map(build_plot_values, values))

    return data


def aggregate() -> str:
    """
    :param data: List of tuples where each represents a line in the plot
                 with tuple beeing (name, values, indices)
    :param fname: Filename of plot

    :returns: Absolut path to saved plot
    """
    group_name = FLAGS.group_name
    dname = storage.create_output_subdir(group_name)
    fname = storage.fname_with_default_dir("plot_task_accuracies.png", dname)

    data = prepare_aggregation_data(group_name)

    # Take highest length of values list as xlim_max
    xlim_max = max([len(values) for _, values, _ in data]) + 1

    fpath = plot(
        data,
        title="Validation set accuracy for unitary and federated learning",
        ylabel="accuracy",
        fname=fname,
        save=True,
        show=False,
        ylim_max=1.0,
        xlim_max=xlim_max,
    )

    logging.info(f"Data plotted and saved in {fpath}")

    return fpath


def app_run_aggregate():
    flags.mark_flag_as_required("group_name")
    app.run(main=lambda _: aggregate())
