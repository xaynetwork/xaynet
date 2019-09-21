import json
from pathlib import Path

import pytest


@pytest.fixture
def results_json_fname(tmpdir):
    results = {
        "hist_opt_configs": [
            [{"learning_rate": 0.1}, {"learning_rate": 0.1}],
            [{"learning_rate": 0.2}, {"learning_rate": 0.2}],
        ]
    }

    fname = Path(tmpdir).joinpath("results.json")

    with open(fname, "w") as outfile:
        json.dump(results, outfile, indent=2, sort_keys=True)

    return str(fname)
