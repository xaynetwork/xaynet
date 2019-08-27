import os

from . import storage


def test_get_abspath_fname_with_absolute_path():
    # Prepare
    fname = "/my/absolute/path/myfile"
    expected_abspath = fname

    # Execute
    actual_abspath = storage.get_abspath(fname)

    # Assert
    assert expected_abspath == actual_abspath


def test_get_abspath_fname_only_filename(output_dir):
    # Prepare
    fname = "myfile"
    expected_abspath = os.path.join(output_dir, fname)

    # Execute
    actual_abspath = storage.get_abspath(fname, output_dir)

    # Assert
    assert expected_abspath == actual_abspath
    assert output_dir in actual_abspath
