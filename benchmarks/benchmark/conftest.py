import pytest
from absl import flags

FLAGS = flags.FLAGS


@pytest.fixture
def group_name():
    gname = "foo_bar_group"
    FLAGS(["test", f"--group_name={gname}"])

    return gname


@pytest.fixture
def results_dir(tmpdir):
    """Create a results_dir containing one file in a subdirectory"""
    tmpdir = str(tmpdir)
    FLAGS(["test", f"--results_dir={tmpdir}"])

    return tmpdir
