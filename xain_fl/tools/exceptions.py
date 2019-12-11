class DuplicatedUpdateError(Exception):
    """Exception raised when the same participant tries to submit multiple
    updates to the :class:`~.Coordinator` in the same :class:`~.Round`
    """


class UnknownParticipantError(Exception):
    """Exception raised when a participant that is unknown to the
    :class:`~.Coordinator` makes a request.

    Typically this means that a participant tries to make a request before it
    has successfully rendezvous with the :class:`~.Coordinator`.
    """


class InvalidRequestError(Exception):
    """Exception raised when the Coordinator receives and invalid request from a participant.

    This can happen if the participant sends a request that is not allowed in a
    give Coordinator state. For instance the Coordinator will only accept
    StartTraining requests during a ROUND.
    """
