"""Provides xain package SDK"""

import sys

from .participant import ParticipantABC, ParticipantError
from .utils import configure_logging


def run_participant(
    participant: ParticipantABC, coordinator_url: str, heartbeat_period: float = 1
):
    from .participant import (  # pylint: disable=import-outside-toplevel
        InternalParticipant,
    )

    internal_participant = InternalParticipant(
        participant, coordinator_url, heartbeat_period
    )
    internal_participant.run()


__all__ = ["configure_logging"]
