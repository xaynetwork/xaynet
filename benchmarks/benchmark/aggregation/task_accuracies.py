import os
from typing import List, Tuple

from absl import flags

from benchmarks.helpers import storage
from xain_fl.logger import get_logger
from xain_fl.types import PlotValues

from .plot import plot
from .results import GroupResult, TaskResult

FLAGS = flags.FLAGS


logger = get_logger(__name__)


def _read_task_values(task_result: TaskResult) -> Tuple[bool, str, List[float], int]:
    """Reads unitary and federated accuracy from results.json.

    Args:
        fname (str): Path to results.json file containing required fields.

    Example:
        >>> print(read_task_values(task_result))
        (false, "VisionTask", [0.12, 0.33], 5)

    Returns:
        ~typing.Tuple[bool, str, ~typing.List[float], int]: Tuple consisting of information,
            if the task is unitary or not, the task label, a list of accuracies and the epochs.
    """
    return (
        task_result.is_unitary(),
        task_result.get_label(),
        task_result.get_accuracies(),
        task_result.get_E(),
    )


def read_all_task_values(group_dir: str) -> List[Tuple[bool, str, List[float], int]]:
    """Reads results directory for given group id and extracts values from results.json files.

    Args:
        group_dir (str): Path to group directory to be read.

    Example:
        >>> print(read_all_task_values(group_dir))
        [(false, "VisionTask", [0.12, 0.33], 5), (true, "UnitaryTask", [0.23, 0.34], 2),...]

    Returns:
        ~typing.List[typing.Tuple[bool, str, ~typing.List[float], int]]: List of tuples consisting
            of information, if the task is unitary or not, the task label, a list of accuracies and
            the epochs.
    """
    task_results = GroupResult(group_dir).get_results()
    # Read accuracies from each file and return list of values in tuples
    return [_read_task_values(task_result) for task_result in task_results]


def build_plot_values(values: Tuple[bool, str, List[float], int]) -> PlotValues:
    """Returns PlotValues with appropriate indices based on task class (Unitary or Federated)"""
    is_unitary, task_label, task_accuracies, E = values

    if is_unitary:
        indices = [i for i in range(1, len(task_accuracies) + 1, 1)]
    else:
        indices = [i for i in range(E, len(task_accuracies) * E + 1, E)]

    return (task_label, task_accuracies, indices)


def _prepare_aggregation_data(group_name: str) -> List[PlotValues]:
    """Constructs and returns curves and xticks_args

    Args:
        group_name (str): group name for which to construct the curves

    Returns:
        A list of `PlotValues`.
    """
    group_dir = os.path.join(FLAGS.results_dir, group_name)
    # List of tuples (benchmark_name, unitary_accuracy, federated_accuracy)
    values = read_all_task_values(group_dir=group_dir)

    assert values, "No values for group found"

    data: List[PlotValues] = list(map(build_plot_values, values))

    return data


def aggregate() -> str:
    """Plots task accuracies for all federated tasks in a group
    Expects FLAGS.group_name to be set

    Returns:
        str: Absolut path to saved plot
    """
    group_name = FLAGS.group_name
    dname = storage.create_output_subdir(group_name)
    fname = storage.fname_with_default_dir("plot_task_accuracies.png", dname)

    data = _prepare_aggregation_data(group_name)

    # Take highest length of values list as xlim_max
    xlim_max = max([len(values) for _, values, _ in data]) + 1

    fpath = plot(
        data,
        title="",  # TODO
        ylabel="Accuracy",
        fname=fname,
        ylim_max=1.0,
        xlim_max=xlim_max,
    )

    logger.info("Data plotted and saved in file", filepath=fpath)

    return fpath
