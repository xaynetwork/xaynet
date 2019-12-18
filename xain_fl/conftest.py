from absl import flags

FLAGS = flags.FLAGS


def pytest_collection_modifyitems(items):
    for item in items:
        if not any(item.iter_markers()):
            item.add_marker("unmarked")


def pytest_runtest_setup():
    # Invoking FLAGS will make the flags usable for the
    # test execution and avoid throwing an error
    FLAGS(
        argv=[
            "test",  # some app name required
            "--fetch_datasets=True",  # resetting to default at beginning of every test
        ]
    )
