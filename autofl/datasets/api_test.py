from . import api


def test_api():
    """Just checks if the api module exports the public methods correctly"""
    public_methods = [
        "cifar10_random_splits_10_load_split",
        "cifar10_random_splits_10_load_splits",
    ]

    for public_method in public_methods:
        assert hasattr(api, public_method)
