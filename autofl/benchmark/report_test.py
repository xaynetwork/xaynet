import hashlib
import os

import pytest
from absl import flags

from . import report

FLAGS = flags.FLAGS


def sha1checksum(fpath: str):
    sha1 = hashlib.sha1()

    with open(fpath, "rb") as f:
        while True:
            data = f.read()
            if not data:
                break
            sha1.update(data)

    return sha1.hexdigest()


def test_get_abspath_fname_with_absolute_path():
    # Prepare
    fname = "/my/absolute/path/myfile"
    expected_abspath = fname

    # Execute
    actual_abspath = report.get_abspath(fname)

    # Assert
    assert expected_abspath == actual_abspath


def test_get_abspath_fname_only_filename(output_dir):
    # Prepare
    fname = "myfile"
    expected_abspath = os.path.join(output_dir, fname)

    # Execute
    actual_abspath = report.get_abspath(fname)

    # Assert
    assert expected_abspath == actual_abspath
    assert output_dir in actual_abspath


@pytest.mark.integration
def test_plot_idd_cpp_comparision(output_dir):
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
    expected_sha1 = "109d91d90a7a746fa1e99cd198d823025db3bd17"

    # Execute
    actual_filepath = report.plot_idd_cpp_comparision(data=data, fname=fname)

    # If any error occurs we will be able to look at the plot. If the the ploting
    # logic is changed the file under this path can be used to get the new hash
    # after evaluating the rendered plot
    print(actual_filepath)

    # Assert
    assert expected_filepath == actual_filepath
    assert expected_sha1 == sha1checksum(actual_filepath), "Checksum not matching"


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
    expected_sha1 = "f8d835bc1a5443ee40fa69dd9e222f61c154f1be"

    # Execute
    actual_filepath = report.plot_accuracies(data=data, fname=fname)

    # If any error occurs we will be able to look at the plot. If the the ploting
    # logic is changed the file under this path can be used to get the new hash
    # after evaluating the rendered plot
    print(actual_filepath)

    # Assert
    assert expected_filepath == actual_filepath
    assert expected_sha1 == sha1checksum(actual_filepath), "Checksum not matching"
