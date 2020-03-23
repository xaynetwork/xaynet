"""Provides xain package SDK"""

import sys

from .interfaces import TrainingInputABC, TrainingResultABC
from .participant import ParticipantABC, ParticipantError


def run_participant(
    participant: ParticipantABC, coordinator_url: str, heartbeat_frequency: float = 1
):
    from .utils import configure_logging  # pylint: disable=import-outside-toplevel
    from .participant import (  # pylint: disable=import-outside-toplevel
        InternalParticipant,
    )

    configure_logging()
    internal_participant = InternalParticipant(
        participant, coordinator_url, heartbeat_frequency
    )
    internal_participant.run()


__all__ = ["TrainingInputABC", "TrainingResultABC"]
