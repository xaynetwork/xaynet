import json
from pathlib import Path

import pytest


@pytest.fixture
def unitary_results_json_fname(tmpdir):
    results = {"partition_id": 0, "hist_opt_configs": None}

    fname = Path(tmpdir).joinpath("results.json")

    with open(fname, "w") as outfile:
        json.dump(results, outfile, indent=2, sort_keys=True)

    return str(fname)


@pytest.fixture
def federated_results_json_fname(tmpdir):
    results = {
        "partition_id": None,
        "hist_opt_configs": [
            [{"learning_rate": 0.1}, {"learning_rate": 0.1}],
            [{"learning_rate": 0.2}, {"learning_rate": 0.2}],
        ],
    }

    fname = Path(tmpdir).joinpath("results.json")

    with open(fname, "w") as outfile:
        json.dump(results, outfile, indent=2, sort_keys=True)

    return str(fname)
