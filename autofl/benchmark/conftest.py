import pytest
from absl import flags

FLAGS = flags.FLAGS


@pytest.fixture
def output_dir(tmpdir):
    od = str(tmpdir)
    FLAGS(["test", f"--output_dir={od}"])
    return od
