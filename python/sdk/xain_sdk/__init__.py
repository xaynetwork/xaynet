"""Provides xain package SDK"""

import sys

from .interfaces import TrainingInputABC, TrainingResultABC
from .participant import ParticipantABC, ParticipantError


def run_participant(
    participant: ParticipantABC, coordinator_url: str,
):
    from .utils import configure_logging
    from .participant import InternalParticipant

    configure_logging()
    internal_participant = InternalParticipant(participant, coordinator_url)
    try:
        internal_participant.run()
    except ParticipantError:
        sys.exit(1)


__all__ = ["TrainingInputABC", "TrainingResultABC"]
