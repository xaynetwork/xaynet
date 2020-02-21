"""XAIN FL Coordinator Session State"""

import threading

from numpy import ndarray

from xain_proto.fl.coordinator_pb2 import State


class Session:
    """Class to manage the state of a federated learning session.

    Args:
    ... TODO
    """

    def __init__(self, state: State, current_round: int, epoch_base: int, weights: ndarray) -> None:
        """TODO"""
        self._state = state
        self._current_round = current_round
        self._epoch_base = epoch_base
        self._weights = weights
        self._lock: threading.Lock = threading.Lock()

    def get_state(self) -> State:
        """TODO"""
        with self._lock:
            return self._state

    def set_state(self, new_state: State) -> None:
        """TODO"""
        with self._lock:
            self._state = new_state

    def get_round(self) -> int:
        """TODO"""
        with self._lock:
            return self._current_round

    def next_round(self) -> None:
        """TODO"""
        with self._lock:
            self._current_round += 1

    def get_epoch_base(self) -> int:
        """TODO"""
        with self._lock:
            return self._epoch_base

    def add_epochs(self, epochs: int) -> None:
        """TODO"""
        with self._lock:
            self._epoch_base += epochs

    def get_weights(self) -> ndarray:
        """TODO"""
        with self._lock:
            return self._weights

    def set_weights(self, weights: ndarray) -> None:
        """TODO"""
        with self._lock:
            self._weights = weights
