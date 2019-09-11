import os

import pytest

from xain.helpers import sha1

from . import task_accuracies


@pytest.mark.integration
def test_plot_task_accuracies(output_dir, group_name, monkeypatch):
    # Prepare
    data = [
        (
            "unitary",
            [0.96, 0.90, 0.81, 0.72, 0.63, 0.54, 0.45, 0.36, 0.27, 0.18, 0.09],
            range(1, 12, 1),
        ),
        (
            "federated",
            [0.92, 0.89, 0.87, 0.85, 0.83, 0.81, 0.80, 0.79, 0.78, 0.77, 0.77],
            range(1, 12, 1),
        ),
    ]
    fname = f"plot_task_accuracies_{group_name}.png"
    expected_filepath = os.path.join(output_dir, fname)
    expected_sha1 = "7138bde2b95eedda6b05b665cc35a6cf204e35e1"

    def mock_prepare_aggregation_data(_: str):
        return data

    monkeypatch.setattr(
        task_accuracies, "prepare_aggregation_data", mock_prepare_aggregation_data
    )

    # Execute
    actual_filepath = task_accuracies.aggregate()

    # If any error occurs we will be able to look at the plot. If the the ploting
    # logic is changed the file under this path can be used to get the new hash
    # after evaluating the rendered plot
    print(actual_filepath)

    # Assert
    assert expected_filepath == actual_filepath
    assert expected_sha1 == sha1.checksum(actual_filepath), "Checksum not matching"
