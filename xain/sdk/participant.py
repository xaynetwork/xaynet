"""Provides participant API"""
from .use_case import UseCase


def start(coordinator_url: str, use_case: UseCase):
    """Starts a participant which will connect to coordinator_url and
    work on use_case

    Args:
        coordinator_url (str): URL of the coordinator to connect to
        use_case (UseCase): Instance of UseCase class
    """

    raise NotImplementedError
