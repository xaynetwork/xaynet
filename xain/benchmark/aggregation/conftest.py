import json
import os
from pathlib import Path

import pytest

# TODO: Use TaskResults class to read/write task results
#       to have one central way of reading/writing results

results_unitary = {
    # Does not contain all keys which are in a actual results.json file
    # just the ones nessecary for the currently existing tests
    "task_name": "unitary",
    "task_label": "unitary label",
    "partition_id": 0,
    "hist_opt_configs": None,
}


results_federated = {
    # Does not contain all keys which are in a actual results.json file
    # just the ones nessecary for the currently existing tests
    "task_name": "federated",
    "task_label": "federated label",
    "partition_id": None,
    "hist_opt_configs": [
        [{"learning_rate": 0.1}, {"learning_rate": 0.1}],
        [{"learning_rate": 0.2}, {"learning_rate": 0.2}],
    ],
}


@pytest.fixture
def group_dir(tmpdir):
    """Group dir with one unitary and one federated task each containing a results.json"""
    task_unitary_dir = Path(tmpdir).joinpath("unitary")
    task_federated_dir = Path(tmpdir).joinpath("federated")

    os.makedirs(task_unitary_dir)
    os.makedirs(task_federated_dir)

    unitary_results_fname = task_unitary_dir.joinpath("results.json")
    federated_results_fname = task_federated_dir.joinpath("results.json")

    with open(unitary_results_fname, "w") as outfile:
        json.dump(results_unitary, outfile, indent=2, sort_keys=True)

    with open(federated_results_fname, "w") as outfile:
        json.dump(results_federated, outfile, indent=2, sort_keys=True)

    return tmpdir


@pytest.fixture
def unitary_results_json_fname(tmpdir):
    fname = Path(tmpdir).joinpath("results.json")

    with open(fname, "w") as outfile:
        json.dump(results_unitary, outfile, indent=2, sort_keys=True)

    return str(fname)


@pytest.fixture
def federated_results_json_fname(tmpdir):
    fname = Path(tmpdir).joinpath("results.json")

    with open(fname, "w") as outfile:
        json.dump(results_federated, outfile, indent=2, sort_keys=True)

    return str(fname)
