import os
from typing import List, Tuple

from absl import app, flags, logging

from xain.types import PlotValues

from .aggregation import GroupResult, TaskResult
from .plot import plot

FLAGS = flags.FLAGS


def read_task_values(task_result: TaskResult) -> Tuple[str, List[float]]:
    """Reads unitary and federated accuracy from results.json

    Args:
        fname (str): path to results.json file containing required fields

    Returns
        class, label, final_accuracy (str, str, float): e.g. ("VisionTask", "cpp01", 0.92)
    """
    return (task_result.get_class(), task_result.get_accuracies())


def read_all_task_values(group_dir: str) -> List[Tuple[str, List[float]]]:
    """
    Reads results directory for given group id and
    extracts values from results.json files

    :param filter_substring: has to be part of the dir name in results directory

    :returns: List of tuples (task_class, task_label, federated_accuracy)
    """
    task_results = GroupResult(group_dir).get_results()
    # Read accuracies from each file and return list of values in tuples
    return [read_task_values(task_result) for task_result in task_results]


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
    assert len(values) == 2, "Expecting only two tasks"

    data: List[PlotValues] = []

    for value in values:
        print(value)
        task_class, task_accuracies = value
        indices = list(range(1, len(task_accuracies) + 1))
        data.append((task_class, task_accuracies, indices))

    return data


def aggregate() -> str:
    """
    :param data: List of tuples where each represents a line in the plot
                 with tuple beeing (name, values, indices)
    :param fname: Filename of plot

    :returns: Absolut path to saved plot
    """
    group_name = FLAGS.group_name
    fname = f"plot_{group_name}.png"

    data = prepare_aggregation_data(group_name)

    # Take highest length of values list as xlim_max
    xlim_max = max([len(values) for _, values, _ in data])

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

    logging.info(f"Data plotted and saved in {fname}")

    return fpath


def app_run_aggregate():
    flags.mark_flag_as_required("group_name")
    app.run(main=lambda _: aggregate())
