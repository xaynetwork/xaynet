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
    try:
        internal_participant.run()
    except ParticipantError:
        sys.exit(1)


__all__ = ["TrainingInputABC", "TrainingResultABC"]
