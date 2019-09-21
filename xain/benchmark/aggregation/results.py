import os
from abc import ABC
from typing import List

from xain.helpers import storage


class TaskResult(ABC):
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

    def get_learning_rates(self) -> List[float]:
        """Will extract learning rate in each round from hist_opt_configs
        which has a List[List[Dict[str, any]]] type. Where the top level
        list contains ROUNDS elements and the second level lists contain
        number of participants dictionaries (one for each participant)"""
        hist_opt_configs = self.data["hist_opt_configs"]

        # We extract the learning rate only from the first participant
        # as the participants share the same learning rate in each round
        learning_rates = [
            participants_in_round[0]["learning_rate"]
            for participants_in_round in hist_opt_configs
        ]

        return learning_rates

    def get_E(self) -> int:
        return self.data["E"]

    def is_unitary(self) -> bool:
        return self.data["partition_id"] is not None


class GroupResult(ABC):
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
        return self.task_results
