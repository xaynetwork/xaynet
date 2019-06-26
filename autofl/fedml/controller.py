import random

from .ops import federated_averaging


class RandomController:
    def __init__(self, num_participants: int, C: int):
        self.num_participants = num_participants
        self.C = C

    def indices(self):
        return random.sample(range(0, self.num_participants), self.C)

    @staticmethod
    def aggregate(thetas):
        theta_prime = federated_averaging(thetas)
        return theta_prime


class RoundRobinController(RandomController):
    def __init__(self, num_participants: int) -> None:
        super().__init__(num_participants, C=1)
