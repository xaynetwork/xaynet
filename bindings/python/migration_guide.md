# Migration from `v0.8.0` to `v.0.11.0`

To demonstrate the API changes from `v0.8.0` to `v.0.11.0`, we will use the keras example
which is available in both versions. For reasons of clarity, some parts of the code have
been removed.

## [`v0.8.0`](https://github.com/xaynetwork/xaynet/blob/v0.8.0/python/sdk/xain_sdk/participant.py#L24)

```bash
pip install xain-sdk
```

```python
from xain_sdk import ParticipantABC, configure_logging, run_participant

class Participant(ParticipantABC):
    def train_round(
        self, training_input: Optional[np.ndarray]
    ) -> Tuple[np.ndarray, int]:
        if training_input is None:
            self.regressor = Regressor(len(self.trainset_x.columns))
            return (self.regressor.get_weights(), 0)

        return (self.regressor.get_weights(), self.number_of_samples)

    def deserialize_training_input(self, data: bytes) -> Optional[np.ndarray]:
        if not data:
            return None

        reader = BytesIO(data)
        return np.load(reader, allow_pickle=False)

    def serialize_training_result(
        self, training_result: Tuple[np.ndarray, int]
    ) -> bytes:
        (weights, number_of_samples) = training_result

        writer = BytesIO()
        writer.write(number_of_samples.to_bytes(4, byteorder="big"))
        np.save(writer, weights, allow_pickle=False)
        return writer.getbuffer()[:]

def main() -> None:
    participant = Participant(args.data_directory)

    run_participant(
        participant, args.coordinator_url, heartbeat_period=args.heartbeat_period
    )
```

## `v0.11.0`

```bash
pip install xaynet-sdk-python
```

```python
# - renamed `run_participant` to `spawn_participant`
# - removed `configure_logging`
from xaynet_sdk import ParticipantABC, spawn_participant

class Participant(ParticipantABC):
    # Returns:
    #   - returns a `np.ndarray` instead of `Tuple[np.ndarray, int]`
    #     The scalar has been moved to the `spawn_participant` function.
    #     This change is only temporary. In a future version it will again
    #     be possible to set the scalar in the `train_round` method.
    def train_round(self, training_input: Optional[np.ndarray]) -> np.ndarray:
        if training_input is None:
            self.regressor = Regressor(len(self.trainset_x.columns))
            return self.regressor.get_weights()

        return self.regressor.get_weights()

    # Args:
    #   - renamed `data` to `global_model`
    #   - provides a `list` instead of `Optional[bytes]`
    #   - `deserialize_training_input` is not called if `global_model` is `None`
    #     therefore the `None` case no longer needs to be handled.
    #
    # Returns:
    #   - returns a `np.ndarray` instead of `Optional[np.ndarray]`
    def deserialize_training_input(self, global_model: list) -> np.ndarray:
        return np.array(global_model)

    # Args:
    #   - provides a `np.ndarray` instead of `Tuple[np.ndarray, int]`
    #
    # Returns:
    #   - returns a `list` instead of `bytes`
    def serialize_training_result(self, training_result: np.ndarray) -> list:
        return training_result.tolist()

def main() -> None:
    # - `spawn_participant` spawns the participant in a separate thread instead of the main thread.
    #
    # Args:
    #   - removed `heartbeat_period`
    #   - `Participant` is instantiated in the participant thread instead of the main thread.
    #     This ensures that both the participant as well as the model of `Participant` live on
    #     the same thread. If they don't live on the same thread, it can cause problems with some
    #     of the ml frameworks.
    participant = spawn_participant(
        args.coordinator_url,
        Participant,
        args=(args.data_directory,)
        scalar = 1 / number_of_samples
    )

    try:
        participant.join()
    except KeyboardInterrupt:
        participant.stop()
```
