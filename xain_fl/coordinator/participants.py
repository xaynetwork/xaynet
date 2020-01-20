"""XAIN FL Participants"""

import threading
import time
from typing import Dict, List

from xain_fl.coordinator import HEARTBEAT_TIME, HEARTBEAT_TIMEOUT


class ParticipantContext:  # pylint: disable=too-few-public-methods
    """Class to store state about each participant. Currently it only stores the `participant_id`
    and the time when the next heartbeat_expires.

    In the future we may store more information like in what state a participant is in e.g.
    `IDLE`, `RUNNING`, ...

    Args:

        participant_id: The id of the participant. Typically a
            host:port or public key when using SSL.
    """

    def __init__(self, participant_id: str) -> None:
        self.participant_id: str = participant_id
        self.heartbeat_expires: float = time.time() + HEARTBEAT_TIME + HEARTBEAT_TIMEOUT


class Participants:
    """This class provides some useful methods to handle all the participants
    connected to a coordinator in a thread safe manner by protecting access to
    the participants list with a lock.
    """

    def __init__(self) -> None:
        self.participants: Dict[str, ParticipantContext] = {}
        self._lock: threading.Lock = threading.Lock()

    def add(self, participant_id: str) -> None:
        """Adds a new participant to the list of participants.

        Args:

            participant_id: The id of the participant to add.
        """

        with self._lock:
            self.participants[participant_id] = ParticipantContext(participant_id)

    def remove(self, participant_id: str) -> None:
        """Removes a participant from the list of participants.

        This will be typically used after a participant is
        disconnected from the coordinator.

        Args:

            participant_id: The id of the participant to remove.
        """

        with self._lock:
            if participant_id in self.participants:
                del self.participants[participant_id]

    def next_expiration(self) -> float:
        """Helper method to check what is the next heartbeat to expire.

        Currently being used by the `heartbeat_monitor` to check how long it should sleep until
        the next check.

        Returns:

            The next heartbeat to expire.
        """

        with self._lock:
            if self.participants:
                return min([p.heartbeat_expires for p in self.participants.values()])

        return time.time() + HEARTBEAT_TIME + HEARTBEAT_TIMEOUT

    def len(self) -> int:
        """Get the number of participants.

        Returns:

            The number of participants in the list.
        """

        with self._lock:
            return len(self.participants)

    def ids(self) -> List[str]:
        """Get the ids of the participants.

        Returns:

            The list of participant ids.
        """

        with self._lock:
            return list(self.participants.keys())

    def update_expires(self, participant_id: str) -> None:
        """Updates the heartbeat expiration time for a participant.

        This is currently called by the :class:`xain_fl.coordinator.coordinator.Coordinator`
        every time a participant sends a heartbeat.

        Args:

            participant_id: The id of the participant to update the expire time.
        """

        with self._lock:
            self.participants[participant_id].heartbeat_expires = (
                time.time() + HEARTBEAT_TIME + HEARTBEAT_TIMEOUT
            )
