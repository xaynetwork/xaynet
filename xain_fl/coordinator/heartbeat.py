"""XAIN FL Hearbeats"""

import threading
import time
from typing import List

from structlog import get_logger

from xain_fl.coordinator.coordinator import Coordinator
from xain_fl.logger import StructLogger

logger: StructLogger = get_logger(__name__)


def monitor_heartbeats(
    coordinator: Coordinator, terminate_event: threading.Event
) -> None:
    """Monitors the heartbeat of participants.

    If a heartbeat expires the participant is removed from the :class:`~.Participants`.

    Note:

        This is meant to be run inside a thread and expects an
        :class:`~threading.Event`, to know when it should terminate.

    Args:

        coordinator: The coordinator to monitor for heartbeats.

        terminate_event: A threading event to signal that this method
            should terminate.
    """

    logger.info("Heartbeat monitor starting...")

    while not terminate_event.is_set():
        participants_to_remove: List[str] = []

        now = time.time()
        for participant in coordinator.participants.participants.values():
            if participant.heartbeat_expires < now:
                participants_to_remove.append(participant.participant_id)

        for participant_id in participants_to_remove:
            coordinator.remove_participant(participant_id)

        next_expiration: float = coordinator.participants.next_expiration() - now

        logger.debug("Monitoring heartbeats", next_expiration=next_expiration)

        # It is better not to use time.sleep() here because if there
        # is no participant,
        # coordinator.participants.next_expiration() can be quite big
        # and we end up sleeping for a long time, even if
        # terminate_event gets set in the meantime.
        terminate_event.wait(timeout=next_expiration)
