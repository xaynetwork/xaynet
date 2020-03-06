# Coordinator

This repo contains a proof of concept implementation of the
coordinator in Rust.


## Architecture

This aggregator handles very different types of requests than the
coordinator itself: weight distribution involves streaming high
volumes of data over the network, and aggregation involves periodic
CPU intensive tasks.

For these reasons, we're trying to make it a separate service that
communicates with the coordinator via a RPC. Here is how we envision
weights distribution with the aggregator service:

![weights distribution sequence diagram](./_images/aggregator_service.png)

Here is a diagram of the various component and how they interact with
each other:

![architecture diagram](./_images/architecture.png)

## Running the Coordinator/ Aggregator locally

### Examples

To run the tensorflow example:

```bash
pip install python/sdk
pip install tensorflow==1.15.2
python python/client_examples/tf.py
```

### Coordinator

```bash
cargo run --bin coordinator -- -c configs/dev-coordinator.toml
```

### Aggregator

The aggregator can be configured to use different backends for
aggregation. Currently, only python aggregators are supported. Some of
these aggregators can be found in `python/aggregators`. In
order to use them that package must be installed:

```bash
pip install python/aggregators/xain_aggregators
# or for development:
pip install -e python/aggregators/xain_aggregators
```

Then the aggregator can be started with:

```bash
cargo run --bin aggregator -- -c configs/dev-aggregator.toml
```

### Docker-compose

```bash
docker-compose -f docker/docker-compose.yml up --build
```

By default, the docker images use debug builds. To use a release build, run:

```bash
docker-compose -f docker/docker-compose.yml -f docker/docker-compose-release.yml up --build
```

To check if the coordinator or the aggregator are leaking memory, run:

```bash
docker-compose -f docker/docker-compose.yml -f docker/docker-compose-valgrind.yml up --build

# To see the logs, run:
docker logs docker_coordinator_1
docker logs docker_aggregator_1
```
