import pytest

from . import coordinator


@pytest.mark.xfail
def test_start():
    coordinator.start()
