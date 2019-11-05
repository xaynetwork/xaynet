import os

from . import storage


def test_fname_with_default_dir_absolute_path():
    # Prepare
    fname = "/my/absolute/path/myfile"
    expected_abspath = fname

    # Execute
    actual_abspath = storage.fname_with_default_dir(fname)

    # Assert
    assert expected_abspath == actual_abspath


def test_fname_with_default_dir_relative_path(output_dir):
    # Prepare
    fname = "myfile"
    expected_abspath = os.path.join(output_dir, fname)

    # Execute
    actual_abspath = storage.fname_with_default_dir(fname, output_dir)

    # Assert
    assert expected_abspath == actual_abspath
    assert output_dir in actual_abspath
