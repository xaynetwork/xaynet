"""XAIN FL conftest"""


def pytest_collection_modifyitems(items):
    """[summary]

    [extended_summary]

    Args:
        items ([type]): [description]
    """

    for item in items:
        if not any(item.iter_markers()):
            item.add_marker("unmarked")
