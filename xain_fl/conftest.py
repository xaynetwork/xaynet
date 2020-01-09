"""XAIN FL conftest"""


def pytest_collection_modifyitems(items):
    """[summary]

    .. todo:: Advance docstrings (https://xainag.atlassian.net/browse/XP-425)

    Args:
        items ([type]): [description]
    """

    for item in items:
        if not any(item.iter_markers()):
            item.add_marker("unmarked")
