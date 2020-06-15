___
**NOTE: Documentation is outdated, needs updating!**
___

## Running the platform

There are two ways to run the backend: using docker, or by compiling
the binaries manually.

### Using `docker-compose`

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

### Building the project manually

The project currently requires rust nightly so the nightly toolchain
must be installed to compile the project.

The `cargo` command can be run either from the `./rust` directory, or
from the repository's root, in which case the `--manifest-path` must
be specified.

The coordinator can be built and started with:

```bash
# If in ./rust
cargo run --bin coordinator -- -c ../configs/dev-coordinator.toml

# Or if at the repo's root
cargo run --bin coordinator --manifest-path rust/Cargo.toml -- -c configs/dev-coordinator.toml
```

The aggregator can be configured to use different backends for
aggregation. Currently, only python aggregators are supported. Some of
these aggregators can be found in `python/aggregators`. In order to
use them that package must be installed:

```bash
pip install python/aggregators
# or for development:
pip install -e python/aggregators
```

Then the aggregator can be started with:

```bash
# If in ./rust
cargo run --bin aggregator -- -c ../configs/dev-aggregator.toml

# Or if at the repo's root
cargo run --bin aggregator --manifest-path rust/Cargo.toml -- -c configs/dev-aggregator.toml
```

## Running the python examples

The examples are under [./python/client_examples](./python/client_examples).

### `dummy.py`

Install the SDK: `pip install -e python/sdk`, then run the example:

```
cd python/client_examples
python dummy.py \
    --number-of-participants 1 \
    --heartbeat-period 0.3 \
    --coordinator-url http://localhost:8081 \
    --model-size 1kB \
    --verbose
```

### `keras_house_prices`

**All the commands in this section are run from the
python/client_examples/keras_house_prices directory.**

1. Install the SDK and the example:

```
pip install -e ../../sdk
pip install -e .
```

2. Download the dataset from Kaggle:
   https://www.kaggle.com/c/house-prices-advanced-regression-techniques/data

3. Extract the data (into
   `python/client_examples/keras_house_prices/data/` here, but the
   location doesn't matter):

```
(cd ./data ; unzip house-prices-advanced-regression-techniques.zip)
```

4. Prepare the data:

```
split-data --data-directory data --number-of-participants 10
```

5. Run one participant:

```
run-participant --data-directory data
```

6. Repeat the previous step to run more participants

Steps 5. and 6. can be combined through the `run.sh` script, which
takes a number of participants to run as argument. To run 10
participants:

```bash
./run.sh 10
```

## Troubleshooting

### py_aggregator.rs tests are failing on macOS

**Error: ModuleNotFoundError: No module named 'xain_aggregators'**

__Solution:__

Make sure that you install the module globally and not within a virtualenv.

```shell
cd python/
pip install aggregators/
```

**Error: ImportError: ... Symbol not found: ...**

__Solution:__

If you installed python via `pyenv` and the error message `Symbol not found` appears, you may have
to reinstall Python with the `--enable-shared` option:

`env PYTHON_CONFIGURE_OPTS="--enable-shared" pyenv install 3.7.6`

Next, run `cargo clean` to remove the old build artifacts. After that, all tests should pass.

[You can find more information about the solution here.](https://github.com/PyO3/pyo3/issues/742#issuecomment-577332616)

**`cargo test` fails with SIGSEGV: invalid memory reference**

This is a known issue with `pyo3`: https://github.com/PyO3/pyo3/issues/288

__Solution:__

Run the tests with:

```none
cargo test -- --test-threads=1
```

