import logging
import threading
from typing import List, Optional

from justbackoff import Backoff

from xaynet_sdk import xaynet_sdk

# rust participant logging
xaynet_sdk.init_logging()
# python participant logging
LOG = logging.getLogger("participant")


class AsyncParticipant(threading.Thread):
    def __init__(
        self,
        coordinator_url: str,
        notifier,
        state,
        scalar,
    ):
        # xaynet rust participant
        self._xaynet_participant = xaynet_sdk.Participant(
            coordinator_url, scalar, state
        )

        self._exit_event = threading.Event()
        self._poll_period = Backoff(min_ms=100, max_ms=10000, factor=1.2, jitter=False)

        # new global model notifier
        self._notifier = notifier

        # calls to an external lib are thread-safe https://stackoverflow.com/a/42023362
        # however, if a user calls `stop` in the middle of the `_tick` call, the
        # `save` method will be executed (which consumes the participant) and every following call
        # will fail with a call on an uninitialized participant. Therefore we lock during `tick`.
        self._tick_lock = threading.Lock()

        super().__init__(daemon=True)

    def run(self):
        try:
            self._run()
        except Exception as err:  # pylint: disable=broad-except
            LOG.error("unrecoverable error: %s shut down participant", err)
            self._exit_event.set()

    def _notify(self):
        if self._notifier.is_set() is False:
            LOG.debug("notify that a new global model is available")
            self._notifier.set()

    def _run(self):
        while not self._exit_event.is_set():
            self._tick()

    def _tick(self):
        with self._tick_lock:
            self._xaynet_participant.tick()
            new_global_model = self._xaynet_participant.new_global_model()
            made_progress = self._xaynet_participant.made_progress()

        if new_global_model:
            self._notify()

        if made_progress:
            self._poll_period.reset()
            self._exit_event.wait(timeout=self._poll_period.duration())
        else:
            self._exit_event.wait(timeout=self._poll_period.duration())

    def get_global_model(self) -> Optional[list]:
        """
        Fetches the current global model. This method can be called at any time. If no global
        model exists (usually in the first round), the method returns `None`.

        Returns:
            The current global model in the form of a list or `None`. The data type of the
            elements match the data type defined in the coordinator configuration.

        Raises:
            GlobalModelUnavailable: If the participant cannot connect to the coordinator to get
                the global model.
            GlobalModelDataTypeMisMatch: If the data type of the global model does not match
                the data type defined in the coordinator configuration.
        """
        LOG.debug("get global model")
        self._notifier.clear()
        with self._tick_lock:
            return self._xaynet_participant.global_model()

    def set_local_model(self, local_model: list):
        """
        Sets a local model. This method can be called at any time. Internally the
        participant first caches the local model. As soon as the participant is selected as an
        update participant, the currently cached local model is used. This means that the cache
        is empty after this operation.

        If a local model is already in the cache and `set_local_model` is called with a new local
        model, the current cached local model will be replaced by the new one.
        If the participant is an update participant and there is no local model in the cache,
        the participant waits until a local model is set or until a new round has been started.

        Args:
            local_model: The local model in the form of a list. The data type of the
                elements must match the data type defined in the coordinator configuration.

        Raises:
            LocalModelLengthMisMatch: If the length of the local model does not match the
                length defined in the coordinator configuration.
            LocalModelDataTypeMisMatch: If the data type of the local model does not match
                the data type defined in the coordinator configuration.
        """
        LOG.debug("set local model in model store")
        with self._tick_lock:
            self._xaynet_participant.set_model(local_model)

    def stop(self) -> List[int]:
        """
        Stops the execution of the participant and returns its serialized state.
        The serialized state can be passed to the `spawn_async_participant` function
        to restore a participant.

        After calling `stop`, the participant is consumed. Every further method
        call on the handle of `AsyncParticipant` leads to an `UninitializedParticipant`
        exception.

        Note:
            The serialized state contains unencrypted **private key(s)**. If used
            in production, it is important that the serialized state is securely saved.

        Returns:
            The serialized state of the participant.
        """
        LOG.debug("stop participant")
        self._exit_event.set()
        self._notifier.clear()
        with self._tick_lock:
            return self._xaynet_participant.save()
