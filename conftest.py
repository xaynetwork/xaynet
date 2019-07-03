"""
This conftest should only contain options for pytest
itself but nothing like fixtures etc.
"""
import pytest
from absl import flags

FLAGS = flags.FLAGS


def pytest_addoption(parser):
    parser.addoption(
        "--runslow", action="store_true", default=False, help="run slow tests"
    )


def pytest_configure(config):
    config.addinivalue_line("markers", "slow: mark test as slow to run")


def pytest_collection_modifyitems(config, items):
    skip_slow = pytest.mark.skip(reason="need --runslow option to run")

    for item in items:
        if not any(item.iter_markers()):
            item.add_marker("unmarked")

        if not config.getoption("--runslow") and "slow" in item.keywords:
            item.add_marker(skip_slow)


def pytest_runtest_setup():
    # Invoking FLAGS will make the flags usable for the
    # test execution and avoid throwing an error
    FLAGS(argv=["test"])
