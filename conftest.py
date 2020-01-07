"""
This conftest should only contain options for pytest
itself but nothing like fixtures etc.
"""
import pytest


def pytest_addoption(parser):
    """[summary]

    [extended_summary]

    Args:
        parser ([type]): [description]
    """

    parser.addoption("--runslow", action="store_true", default=False, help="run slow tests")


def pytest_configure(config):
    """[summary]

    [extended_summary]

    Args:
        config ([type]): [description]
    """

    config.addinivalue_line("markers", "slow: mark test as slow to run")


def pytest_collection_modifyitems(config, items):
    """[summary]

    [extended_summary]

    Args:
        config ([type]): [description]
        items ([type]): [description]
    """

    skip_slow = pytest.mark.skip(reason="need --runslow option to run")

    for item in items:
        if not any(item.iter_markers()):
            item.add_marker("unmarked")

        if not config.getoption("--runslow") and "slow" in item.keywords:
            item.add_marker(skip_slow)
