"""XAIN FL exceptions"""


class DuplicatedUpdateError(Exception):
    """Exception for resubmitted updates.

    Raised when the same participant tries to submit multiple updates to the coordinator
    in the same round.
    """


class UnknownParticipantError(Exception):
    """Exception for unknown participants.

    Raised when a participant that is unknown to the coordinator makes a request.
    Typically this means that a participant tries to make a request before it has
    successfully rendezvous'd with the coordinator.
    """


class InvalidRequestError(Exception):
    """Exception for invalid requests.

    Raised when the Coordinator receives and invalid request from a participant. This
    can happen if the participant sends a request that is not allowed in a given
    coordinator state. For instance the coordinator will only accept StartTrainingRound
    requests during a ROUND.
    """


class InvalidConfigError(Exception):
    """Exception for invalid configurations.

    Raised upon trying to load an invalid configuration.
    """
