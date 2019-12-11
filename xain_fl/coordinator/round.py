from typing import Dict, List, Tuple

from numpy import ndarray

from xain_fl.tools.exceptions import DuplicatedUpdateError


class Round:
    """Class to manage the state of a single round.

    This class contains the logic to handle all updates sent by the
    participants during a round and does some sanity checks like preventing the
    same participant to submit multiple updates during a single round.

    Args:
        required_participants (int): The minimum number of participants required to
            perform a round.
    """

    def __init__(self, required_participants: int) -> None:
        self.required_participants: int = required_participants
        self.updates: Dict[str, Dict] = {}

    def add_updates(
        self,
        participant_id: str,
        weight_update: Tuple[List[ndarray], int],
        metrics: Dict[str, List[ndarray]],
    ) -> None:
        """Valid a participant's update for the round.

        Args:
            participant_id (:obj:`str`): The id of the participant making the request.
            weight_update (:obj:`tuple` of :obj:`list` of :class:`~numpy.ndarray`):
                A tuple containing a list of updated weights.
            metrics (:obj:`dict`): A dictionary containing metrics with the name and the value
                as list of ndarrays.

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
            :obj:`bool`:: :obj:`True` if all participants submitted their
            updates this round. :obj:`False` otherwise.
        """
        return len(self.updates) == self.required_participants

    def get_weight_updates(self) -> List[Tuple[List[ndarray], int]]:
        """Get a list of all participants weight updates.

        This list will usually be used by the aggregation function.

        Returns:
            :obj:`list` of :obj:`tuple`: The list of weight updates from all
            participants.
        """
        return [v["weight_update"] for k, v in self.updates.items()]
