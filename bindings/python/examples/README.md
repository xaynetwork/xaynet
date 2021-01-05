# Examples

Some examples that show how the `ParticipantABC` or `AsyncParticipant` can be used.

## Getting Started

All examples in this section work without changing the coordinator
[config.toml](../../../configs/config.toml) or [docker-dev.toml](../../../configs/docker-dev.toml).

- [`hello_world.py`](./hello_world.py) A basic `ParticipantABC` example
- [`hello_world_async.py`](./hello_world_async.py) A basic `AsyncParticipant` example
- [`download_global_model.py`](./download_global_model.py) A `ParticipantABC` that only downloads the latest global model
- [`download_global_model_async.py`](./download_global_model_async.py) An `AsyncParticipant` that only downloads the latest global model
- [`multiple_participants.py`](./download_global_model_async.py) Spawn multiple `ParticipantABC`s in a single process
- [`participate_in_update.py`](./participate_in_update.py) Only train a model when there is enough battery left
- [`restore.py`](./restore.py) Save and restore the state of an `AsyncParticipant`

## Keras House Prices

- [`keras_house_prices`](./keras_house_prices/) A full machine learning example
