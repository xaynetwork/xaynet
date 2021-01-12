import threading
from typing import List, Optional, Tuple

from .async_participant import *
from .participant import *


def spawn_participant(
    coordinator_url: str,
    participant: ParticipantABC,
    args: Tuple = (),
    kwargs: dict = {},
    state: Optional[List[int]] = None,
    scalar: float = 1.0,
):
    """
    Spawns a `InternalParticipant` in a separate thread and returns a participant handle.
    If a `state` is passed, this state is restored, otherwise a new `InternalParticipant`
    is created.

    Args:
        coordinator_url: The url of the coordinator.
        participant: A class that implements `ParticipantABC`.
        args: The args that get passed to the constructor of the `participant` class.
        kwargs: The kwargs that get passed to the constructor of the `participant` class.
        state: A serialized participant state. Defaults to `None`.
        scalar: The scalar used for masking. Defaults to `1.0`.

    Note:
        The `scalar` is used later when the models are aggregated in order to scale their weights.
        It can be used when you want to weight the participants updates differently.

        For example:
        If not all participant updates should be weighted equally but proportionally to their
        training samples, the scalar would be set to `scalar = 1 / number_of_samples`.

    Returns:
        The `InternalParticipant`.

    Raises:
        CryptoInit: If the initialization of the underling crypto library has failed.
        ParticipantInit: If the participant cannot be initialized. This is most
            likely caused by an invalid `coordinator_url`.
        ParticipantRestore: If the participant cannot be restored due to invalid
            serialized state. This exception can never be thrown if the `state` is `None`.
        Exception: Any exception that can be thrown during the instantiation of `participant`.
    """
    internal_participant = InternalParticipant(
        coordinator_url, participant, args, kwargs, state, scalar
    )
    # spawns the internal participant in a thread.
    # `start` calls the `run` method of `InternalParticipant`
    # https://docs.python.org/3.8/library/threading.html#threading.Thread.start
    # https://docs.python.org/3.8/library/threading.html#threading.Thread.run
    internal_participant.start()
    return internal_participant


def spawn_async_participant(
    coordinator_url: str, state: Optional[List[int]] = None, scalar: float = 1.0
) -> (AsyncParticipant, threading.Event):
    """
    Spawns a `AsyncParticipant` in a separate thread and returns a participant handle
    together with a global model notifier. If a `state` is passed, this state is restored,
    otherwise a new participant is created.

    Args:
        coordinator_url: The url of the coordinator.
        state: A serialized participant state. Defaults to `None`.
        scalar: The scalar used for masking. Defaults to `1.0`.

    Note:
        The `scalar` is used later when the models are aggregated in order to scale their weights.
        It can be used when you want to weight the participants updates differently.

        For example:
        If not all participant updates should be weighted equally but proportionally to their
        training samples, the scalar would be set to `scalar = 1 / number_of_samples`.

    Returns:
        A tuple which consists of an `AsyncParticipant` and a global model notifier.

    Raises:
        CryptoInit: If the initialization of the underling crypto library has failed.
        ParticipantInit: If the participant cannot be initialized. This is most
            likely caused by an invalid `coordinator_url`.
        ParticipantRestore: If the participant cannot be restored due to invalid
            serialized state. This exception can never be thrown if the `state` is `None`.
    """
    notifier = threading.Event()
    async_participant = AsyncParticipant(coordinator_url, notifier, state, scalar)
    async_participant.start()
    return (async_participant, notifier)
