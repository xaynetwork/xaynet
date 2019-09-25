"""Functions for getting participant selection history"""

import os
from typing import List, Tuple

import numpy as np
from absl import app, flags, logging
from numpy import ndarray

from xain.benchmark.aggregation.plot import plot_history_data
from xain.benchmark.aggregation.results import GroupResult, TaskResult
from xain.helpers.storage import create_output_subdir, fname_with_default_dir
from xain.types import Metrics

FLAGS = flags.FLAGS


def participant_history() -> List[str]:
    """Plot participant selection histories for group name flag.

    For each task result in the group name flag extract the task metrics (number of
    participants, task label, hist metrics), transform them into heatmap data as participant
    indices x training rounds and plot/save them as participant selection history.

    Returns:
        ~typing.List[str]: File paths for saved plots.
    """

    group_name: str = FLAGS.group_name
    dir_name: str = create_output_subdir(dname=group_name)
    file_pre_name: str = fname_with_default_dir(
        fname="plot_participant_history_{}.png", dname=dir_name
    )
    file_paths: List[str] = list()

    # Getting history metrics data from results.json
    hist_metrics_group: List[Tuple[int, str, List[List[Metrics]]]] = get_hist_metrics(
        group_name=group_name
    )

    # Creates heatmap data for each task metric in group metrics
    matrices: List[Tuple[str, ndarray]] = list(map(heatmap_data, hist_metrics_group))

    for task_matrix in matrices:
        label: str = task_matrix[0]
        matrix: ndarray = task_matrix[1]

        file_path: str = plot_history_data(
            matrix=matrix,
            title="Participant Selection History",
            file_name=file_pre_name.format(label),
            save=True,
            show=False,
        )
        file_paths.append(file_path)

    logging.info(f"Task data plotted and saved in {file_paths}")

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


def app_run_participant_hist():
    flags.mark_flag_as_required("group_name")
    app.run(main=lambda _: participant_history())
