"""Provides xain package SDK"""

from .participant import Participant, ParticipantError
import sys


def run_participant(participant: Participant, coordinator_url: str):
    from .utils import configure_logging
    from .participant import InternalParticipant

    configure_logging()
    internal_participant = InternalParticipant(participant, coordinator_url)
    try:
        internal_participant.run()
    except ParticipantError:
        sys.exit(1)
