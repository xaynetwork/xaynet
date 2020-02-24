"""XAIN FL Participants"""

import threading
import time
from typing import Dict, List


class ParticipantContext:  # pylint: disable=too-few-public-methods
    """Class to store state about each participant.

    Currently it only stores the id of the participant and the time when the next
    heartbeat expires. In the future we may store more information like in what state a
    participant is in e.g. `IDLE`, `RUNNING`, ...

    Args:
        participant_id: The id of the participant. Typically a host:port or public key
            when using SSL.
        heartbeat_expires: The point in time when the heartbeat will expire.
    """

    def __init__(
        self, participant_id: str, heartbeat_time: int, heartbeat_timeout: int
    ) -> None:
        self.participant_id: str = participant_id
        self.heartbeat_expires: float = time.time() + heartbeat_time + heartbeat_timeout


class Participants:
    """Manage the connected participants.

    This class provides some useful methods to handle all the participants connected to
    a coordinator in a thread safe manner by protecting access to the participants list
    with a lock.
    """

    def __init__(self, heartbeat_time: int = 10, heartbeat_timeout: int = 5) -> None:
        self.participants: Dict[str, ParticipantContext] = {}
        self._lock: threading.Lock = threading.Lock()
        self.heartbeat_time = heartbeat_time
        self.heartbeat_timeout = heartbeat_timeout

    def add(self, participant_id: str) -> None:
        """Add a new participant to the list of participants.

        Args:
            participant_id: The id of the participant to add.
        """

        with self._lock:
            self.participants[participant_id] = ParticipantContext(
                participant_id, self.heartbeat_time, self.heartbeat_timeout
            )

    def remove(self, participant_id: str) -> None:
        """Remove a participant from the list of participants.

        This will be typically used after a participant is disconnected from the
        coordinator.

        Args:
            participant_id: The id of the participant to remove.
        """

        with self._lock:
            if participant_id in self.participants:
                del self.participants[participant_id]

    def next_expiration(self) -> float:
        """Helper method to check what is the next heartbeat to expire.

        Currently being used by the `heartbeat_monitor` to check how long it should
        sleep until the next check.

        Returns:
            The next heartbeat to expire.
        """

        with self._lock:
            if self.participants:
                return min([p.heartbeat_expires for p in self.participants.values()])

        return time.time() + self.heartbeat_time + self.heartbeat_timeout

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
        """Update the heartbeat expiration time for a participant.

        This is currently called by the coordinator every time a participant sends a
        heartbeat.

        Args:
            participant_id: The id of the participant to update the expire time.
        """

        with self._lock:
            self.participants[participant_id].heartbeat_expires = (
                time.time() + self.heartbeat_time + self.heartbeat_timeout
            )
