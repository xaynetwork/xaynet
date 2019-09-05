import os
from abc import ABC
from typing import Dict, List

from absl import logging

from xain.helpers import storage


def flul_aggregation():
    logging.info("flul_aggregation started")
    raise NotImplementedError()


def cpp_aggregation():
    logging.info("cpp_aggregation started")
    raise NotImplementedError()


class TaskResult(ABC):
    def __init__(self, fname: str):
        print(fname)
        self.data = storage.read_json(fname)

    def get_class(self) -> str:
        return self.data["task_name"].split("_")[0]

    def get_label(self) -> str:
        return self.data["dataset"].split("-")[-1]

    def get_final_accuracy(self) -> float:
        return self.data["acc"]

    def get_accuracies(self) -> List[float]:
        return self.data["hist"]["val_acc"]


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
