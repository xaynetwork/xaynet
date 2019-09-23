import os

import pytest
from absl import flags

from xain.helpers import sha1

from . import learning_rate

FLAGS = flags.FLAGS


@pytest.mark.integration
def test_read_all_task_values(group_dir):
    # Prepare
    expected_results = [("federated label", [0.1, 0.2])]

    # Execute
    actual_results = learning_rate.read_all_task_values(group_dir)

    # Assert
    assert actual_results == expected_results


@pytest.mark.integration
def test_plot_learning_rate(output_dir, group_name, monkeypatch):
    # Prepare
    data = [
        ("federated 1 - label", [0.10, 0.05, 0.03, 0.02], [1, 2, 3, 4]),
        ("federated 2 - label", [0.09, 0.07, 0.06, 0.04], [1, 2, 3, 4]),
    ]
    fname = f"plot_learning_rates.png"
    expected_filepath = os.path.join(output_dir, group_name, fname)
    expected_sha1 = "599daf7563f41289a8eed2b59aba3c5d312eeab1"

    def mock_prepare_aggregation_data(_: str):
        return data

    monkeypatch.setattr(
        learning_rate, "prepare_aggregation_data", mock_prepare_aggregation_data
    )

    # Execute
    actual_filepath = learning_rate.aggregate()

    # If any error occurs we will be able to look at the plot. If the the ploting
    # logic is changed the file under this path can be used to get the new hash
    # after evaluating the rendered plot
    print(actual_filepath)

    # Assert
    assert expected_filepath == actual_filepath
    assert expected_sha1 == sha1.checksum(actual_filepath), "Checksum not matching"
