"""Provides the classes TaskResult and GroupResult which wrap
the results of a benchmark group and provide easy access to the
results contained in the results.json files in each tasks results
"""
import os
from abc import ABC
from typing import List, Optional, cast

from benchmarks.helpers import storage
from xain_fl.types import Metrics


class TaskResult(ABC):
    """Provides predictable access to task results"""

    def __init__(self, fname: str):
        self.data = storage.read_json(fname)

    def get_name(self) -> str:
        return self.data["task_name"]

    def get_label(self) -> str:
        return self.data["task_label"]

    def get_final_accuracy(self) -> float:
        return self.data["acc"]

    def get_accuracies(self) -> List[float]:
        return self.data["hist"]["val_acc"]

    def get_learning_rates(self) -> Optional[List[float]]:
        """Will extract learning rate for federated task results in each round
        from hist_opt_configs which has a List[List[Dict[str, any]]] type. The
        top level list contains ROUNDS elements and the second level lists
        contain number of participants dictionaries (one for each participant).
        As unitary task results don't have a history of optimizer configs
        None will be returned.
        """
        if self.is_unitary():
            return None

        hist_opt_configs = self.data["hist_opt_configs"]

        # Extract learning rate only from the first participant as the participants
        # share the same learning rate in each round
        learning_rates = [
            participants_in_round[0]["learning_rate"]
            for participants_in_round in hist_opt_configs
        ]

        return learning_rates

    def get_E(self) -> int:
        return self.data["E"]

    def is_unitary(self) -> bool:
        return self.data["partition_id"] is not None

    def get_hist_metrics(self) -> List[List[Metrics]]:
        """Get history metrics from a task result.

        Extracts history metrics for each training round as list of
        participant indice and VolumeByClass values.

        Returns:
            ~typing.List[~typing.List[Metrics]]: List of hist metrics for each training round.
        """

        hist_metrics = [
            [tuple(metric) for metric in round_metric]
            for round_metric in self.data["hist_metrics"]
        ]
        # mypy is not able to handle list comprehension here correctly
        return cast(List[List[Metrics]], hist_metrics)

    def get_num_participants(self) -> int:
        """Get the number of participants.

        Returns:
            int: Number of participants.
        """

        return self.data["num_participants"]


class GroupResult(ABC):
    """Provides predictable access to the results of all tasks in a group"""

    def __init__(self, group_dir: str):
        assert os.path.isdir(group_dir)

        # get list of all directories which contain given substring
        json_files = [
            fname
            for fname in storage.listdir_recursive(group_dir, relpath=False)
            if fname.endswith("results.json")
        ]

        if not json_files:
            raise Exception(f"No values results found in group_dir: {group_dir}")

        self.task_results = [TaskResult(fname) for fname in json_files]

    def get_results(self) -> List[TaskResult]:
        """Provides a list of TaskResult instances which will enable easy
        access to the results of a benchmark scenario

        Returns:
            List[TaskResult]: Each item in the list corrosponds to one task in the
                benchmark scenario
        """
        return self.task_results
