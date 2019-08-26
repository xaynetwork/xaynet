import os

import pytest
from absl import flags

from autofl.helpers.sha1 import checksum

from . import report

FLAGS = flags.FLAGS


@pytest.mark.integration
def test_plot_iid_noniid_comparison(output_dir):
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
    fname = "myplot.png"
    expected_filepath = os.path.join(output_dir, fname)
    expected_sha1 = "4b9fb44d7d3f92889ada5d59bb74d21a34a5fdaa"

    xticks_locations = range(1, 12, 1)
    xticks_labels = [chr(i) for i in range(65, 77, 1)]  # A, B, ..., K

    # Execute
    actual_filepath = report.plot_iid_noniid_comparison(
        data=data, xticks_args=(xticks_locations, xticks_labels), fname=fname
    )

    # If any error occurs we will be able to look at the plot. If the the ploting
    # logic is changed the file under this path can be used to get the new hash
    # after evaluating the rendered plot
    print(actual_filepath)

    # Assert
    assert expected_filepath == actual_filepath
    assert expected_sha1 == checksum(actual_filepath), "Checksum not matching"


@pytest.mark.integration
def test_plot_accuracies(output_dir):
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
    fname = "myplot.png"
    expected_filepath = os.path.join(output_dir, fname)
    expected_sha1 = "457baa8179f08f06c4e60213eb0bbbe79a4f9d3e"

    # Execute
    actual_filepath = report.plot_accuracies(data=data, fname=fname)

    # If any error occurs we will be able to look at the plot. If the the ploting
    # logic is changed the file under this path can be used to get the new hash
    # after evaluating the rendered plot
    print(actual_filepath)

    # Assert
    assert expected_filepath == actual_filepath
    assert expected_sha1 == checksum(actual_filepath), "Checksum not matching"
