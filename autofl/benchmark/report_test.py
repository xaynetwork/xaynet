import os

from absl import flags

from . import report

FLAGS = flags.FLAGS


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
    expected_abspath = os.path.join(FLAGS.output_dir, fname)

    # Execute
    actual_abspath = report.get_abspath(fname)

    # Assert
    assert expected_abspath == actual_abspath
    assert output_dir in actual_abspath
