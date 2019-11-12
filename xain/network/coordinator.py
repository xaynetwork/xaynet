import math
import random
from typing import List, Tuple

import numpy as np
from numproto import ndarray_to_proto, proto_to_ndarray

from xain.network import stream_pb2
from xain.network.server import ParticipantProxy, create_participant_manager


class Participant:
    """Holds request until its anwered"""

    def __init__(self):
        self.proxy = ParticipantProxy()

    def train(self, theta):
        instruction = stream_pb2.CoordinatorMessage(
            train_config=stream_pb2.CoordinatorMessage.TrainConfig(
                theta=[ndarray_to_proto(nda) for nda in theta]
            )
        )

        res = self.proxy.run(instruction)
        theta = [proto_to_ndarray(nda) for nda in res.result.theta]

        return theta

    def reconnect(self, secs):
        instruction = stream_pb2.CoordinatorMessage(reconnect_in=secs)
        self.proxy.run(instruction, skip_response=True)
        self.proxy.close()


def participant_factory():
    return Participant()


def aggregate(thetas: List[List[np.ndarray]]):
    s = thetas[0][0].shape
    v = thetas[0][0][0] + 1

    return [np.full(s, v), np.full(s, v), np.full(s, v)]


def fit():
    rounds = 10
    C = 0.6
    num_participants = 3
    num_required_participants = math.ceil(num_participants * C)

    participant_manager = create_participant_manager(
        participant_factory=participant_factory
    )

    theta = [np.ones((1, 1)), np.ones((1, 1)), np.ones((1, 1))]

    for i in range(rounds):
        print(f"Starting round: {i+1}/{rounds}")
        print("Waiting for participants")

        participants = participant_manager.get_participants(
            min_num_participants=num_participants
        )

        # Randomly select {num_required_participants} participants
        selected_participants = random.sample(participants, num_required_participants)
        rejected_participants = [
            p for p in participants if p not in selected_participants
        ]

        print(selected_participants, rejected_participants)

        for p in rejected_participants:
            p.reconnect(2)

        theta_updates = fit_round(participants=selected_participants, theta=theta)

        theta = aggregate(theta_updates)
        print("Round result:", theta)

    print("Final result", theta)


def fit_round(participants, theta):
    theta_updates: List[Tuple[np.ndarray]] = []

    for i, p in enumerate(participants):
        print(f"participants[{i}].train()")
        theta_update = p.train(theta)
        theta_updates.append(theta_update)

    return theta_updates


def main():
    fit()


if __name__ == "__main__":
    main()
