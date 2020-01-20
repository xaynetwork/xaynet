"""XAIN FL Rounds"""

from typing import Dict, List, Tuple

from numpy import ndarray

from xain_fl.tools.exceptions import DuplicatedUpdateError


class Round:
    """Class to manage the state of a single round.
    This class contains the logic to handle all updates sent by the
    participants during a round and does some sanity checks like preventing the
    same participant to submit multiple updates during a single round.

    Args:

        participant_ids: The list of IDs of the participants selected
            to participate in this round.
    """

    def __init__(self, participant_ids: List[str]) -> None:
        self.participant_ids = participant_ids
        self.updates: Dict[str, Dict] = {}

    def add_updates(
        self,
        participant_id: str,
        weight_update: Tuple[List[ndarray], int],
        metrics: Dict[str, ndarray],
    ) -> None:
        """Valid a participant's update for the round.

        Args:

            participant_id: The id of the participant making the request.

            weight_update: A tuple containing a list of updated weights.

            metrics: A dictionary containing metrics with the name and
                the value as list of ndarrays.

        Raises:

            DuplicatedUpdateError: If the participant already submitted his update this round.
        """

        if participant_id in self.updates.keys():
            raise DuplicatedUpdateError(
                f"Participant {participant_id} already submitted the update for this round."
            )

        self.updates[participant_id] = {
            "weight_update": weight_update,
            "metrics": metrics,
        }

    def is_finished(self) -> bool:
        """Check if all the required participants submitted their updates this round.
        If all participants submitted their updates the round is considered finished.

        Returns:

            `True` if all participants submitted their updates this
            round. `False` otherwise.
        """
        return len(self.updates) == len(self.participant_ids)

    def get_weight_updates(self) -> List[Tuple[List[ndarray], int]]:
        """Get a list of all participants weight updates.
        This list will usually be used by the aggregation function.

        Returns:

            The list of weight updates from all participants.
        """
        return [v["weight_update"] for k, v in self.updates.items()]
