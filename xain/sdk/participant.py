from .use_case import UseCase


def start(coordinator_url: str, use_case: UseCase):
    print(coordinator_url)
    print(use_case)
