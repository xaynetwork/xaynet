# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to the [Semantic Versioning](http://semver.org/spec/v2.0.0.html).

## [unreleased]

## [0.11.0] - 2021-01-18

### Added

#### Rust SDK `xaynet-sdk`

`xaynet-sdk` contains the basic building blocks required to run the _Privacy-Enhancing Technology_
(PET) Protocol. It consists of a state machine and two I/O interfaces with which specific Xaynet
participants can be developed that are adapted to the respective environments/requirements.

If you are interested in building your own Xaynet participant, you can take a look at
`xaynet-sdk`, our [Rust participant](https://github.com/xaynetwork/xaynet/blob/master/rust/examples/test-drive/participant.rs)
which we use primarily for testing or at
[`xaynet-mobile`](https://github.com/xaynetwork/xaynet/blob/master/rust/xaynet-mobile/src/participant.rs)
our mobile friendly participant.

#### A Mobile friendly Xaynet participant `xaynet-mobile`

`xaynet-mobile` provides a mobile friendly implementation of a Xaynet participant. It gives the user
a lot of control on how to drive the participant execution. You can regularly pause the execution of
the participant, save it, and later restore it and continue the execution. When running on a device
that is low on battery or does not have access to Wi-Fi for instance, it can be useful to be able to
pause the participant.

**C API**

Furthermore, `xaynet-mobile` offers `C` bindings that allow `xaynet-mobile` to be used in other
programming languages ​​such as `Dart`.

#### Python participant SDK `xaynet-sdk-python`

We are happy to announce that we finally released `xaynet-sdk-python` a Python SDK that
consists of two experimental Xaynet participants (`ParticipantABC` and `AsyncParticipant`).

The `ParticipantABC` API is similar to the old one which we introduced in `v0.8.0`. Aside from some
changes to the method signature, the biggest change is that the participant now runs in its own
thread. To migrate from `v0.8.0` to `v0.11.0` please follow the
[migration guide](https://github.com/xaynetwork/xaynet/blob/master/bindings/python/migration_guide.md).

However, we noticed that our Participant API may be difficult to integrate with existing
applications, considering the code for the training has to be moved into the `train_round` method,
which can lead to significant changes to the existing code. Therefore, we offer a second API
(`AsyncParticipant`) in which the training of the model is no longer part of the participant.

A more in-depth explanation of the differences between the Participant APIs
and examples of how to use them can be found
[here](https://github.com/xaynetwork/xaynet/blob/master/bindings/python/README.md).

#### Multi-part messages

Participant messages can get large, possibly too large to be sent successfully in one go. On mobile
devices in particular, the internet connection may not be as reliable. In order to make the
transmission of messages more robust, we implemented multi-part messages to break a large message
into parts and send them sequentially to the coordinator. If the transmission of part of
a message fails, only that part will be resent and not the entire message.

#### Coordinator state managed in Redis

In order to be able to restore the state of the coordinator after a failure or shutdown,
the state is managed in Redis and no longer in memory.

The Redis client can be configured via the `[redis]` setting:

```toml
[redis]
url = "redis://127.0.0.1/"
```

#### Support for storing global models in S3/Minio

The coordinator is able to save a global model in S3/Minio after a successful round.

The S3 client can be configured via the `[s3]` setting:

```toml
[s3]
access_key = "minio"
secret_access_key = "minio123"
region = ["minio", "http://localhost:9000"]

[s3.buckets]
global_models = "global-models"
```

`xaynet-server` must be compiled with the feature flag `model-persistence` in order to enable
this feature.

#### Restore coordinator state

The state of the coordinator can be restored after a failure or shutdown.

Restoring the coordinator be configured via the `[restore]` setting:

```toml
[restore]
enable = true
```

`xaynet-server` must be compiled with the feature flag `model-persistence` in order to enable
this feature.

#### Improved collection of state machine metrics

In `v0.10.0` we introduced the collection of metrics that are emitted in the state machine of
`xaynet-server` and sent to an InfluxDB instance. In `v0.11.0` we have revised the implementation
and improved it further. Metrics are now sent much faster and adding metrics to the code has
become much easier.

### Removed

  - `xaynet_client` (was split into `xaynet_sdk` and `xaynet_mobile`)
  - `xaynet_ffi` (is now part of `xaynet_mobile`)
  - `xaynet_macro`

## [0.10.0] - 2020-09-22

### Added

- Preparation for redis support: prepare for `xaynet_server` to store PET data in redis [#416](https://github.com/xaynetwork/xaynet/pull/416), [#515](https://github.com/xaynetwork/xaynet/pull/515)
- Add support for multipart messages in the message structure [#508](https://github.com/xaynetwork/xaynet/pull/508), [#513](https://github.com/xaynetwork/xaynet/pull/513), [#514](https://github.com/xaynetwork/xaynet/pull/514)
- Generalised scalar extension [#496](https://github.com/xaynetwork/xaynet/pull/496), [#507](https://github.com/xaynetwork/xaynet/pull/507)
- Add server metrics [#487](https://github.com/xaynetwork/xaynet/pull/487), [#488](https://github.com/xaynetwork/xaynet/pull/488), [#489](https://github.com/xaynetwork/xaynet/pull/489), [#493](https://github.com/xaynetwork/xaynet/pull/493)
- Refactor the client into a state machine, and add a client tailored for mobile devices [#471](https://github.com/xaynetwork/xaynet/pull/471), [#497](https://github.com/xaynetwork/xaynet/pull/497), [#506](https://github.com/xaynetwork/xaynet/pull/506)

### Changed

- Split the xaynet crate into several sub-crates:
  - `xaynet_core` (0.1.0 released), re-exported as `xaynet::core`
  - `xaynet_client` (0.1.0 released), re-exported as `xaynet::client` when compiled with `--features client`
  - `xaynet_server` (0.1.0 released), re-exported as `xaynet::server` when compiled with `--features server`
  - `xaynet_macro` (0.1.0 released)
  - `xaynet_ffi` (not released)

## [0.9.0] - 2020-07-24

`xain/xain-fl` repository was renamed to `xaynetwork/xaynet`.

The new crate will be published as `xaynet` under `v0.9.0`.

### Added

This release introduces the integration of the [PET protocol](https://uploads-ssl.webflow.com/5f0c5c0bb18a279f0a62919e/5f157004da6585f299fa542b_XayNet%20Whitepaper%202.1.pdf) into the platform.

**Note:**
The integration of the PET protocol required a complete rewrite of the codebase and is therefore not compatible with the previous release.

## [0.8.0] - 2020-04-08

### Added

- New tutorial for the Python SDK [#355](https://github.com/xaynetwork/xaynet/pull/355)
- Swagger description of the REST API [#345](https://github.com/xaynetwork/xaynet/pull/345), and is published at https://xain-fl.readthedocs.io/en/latest/ [#358](https://github.com/xaynetwork/xaynet/pull/358)
- The Python examples now accepts additional parameters (model size, heartbeat period, verbosity, etc.) [#351](https://github.com/xaynetwork/xaynet/pull/351)
- Publish docker images to dockerhub

### Security

- Stop using `pickle` for messages serialization
  [#355](https://github.com/xaynetwork/xaynet/pull/355). `pickle` is insecure
  and can lead to remote code execution. Instead, the default
  aggregator uses `numpy.save()`.

### Fixed

- The documentation has been updated at https://xain-fl.readthedocs.io/en/latest/ [#358](https://github.com/xaynetwork/xaynet/pull/358)
- Document aggregator error on Darwin platform [#365](https://github.com/xaynetwork/xaynet/pull/365/files)

### Changed

- Simplified the Python SDK API [#355](https://github.com/xaynetwork/xaynet/pull/355)
- Added unit tests for the coordinator and aggregator [#353](https://github.com/xaynetwork/xaynet/pull/353), [#352](https://github.com/xaynetwork/xaynet/pull/352)
- Refactor the metrics store [#340](https://github.com/xaynetwork/xaynet/pull/340)
- Speed up the docker builds [#348](https://github.com/xaynetwork/xaynet/pull/348)

## [0.7.0] - 2020-03-25

On this release we archived the Python code under the `legacy` folder and shifted the development to Rust.
This release has many breaking changes from the previous versions.
More details will be made available through the updated README.md of the repository.

## [0.6.0] - 2020-02-26

- HOTFIX add disclaimer (#309) [janpetschexain]
- PB-314: document the new weight exchange mechanism (#308) [Corentin Henry]
- PB-407 add more debug level logging (#303) [janpetschexain]
- PB-44 add heartbeat time and timeout to config (#305) [Robert Steiner]
- PB-423 lock round access (#304) [kwok]
- PB-439 Make thread pool workers configurable (#302) [Robert Steiner]
- PB-159: update xain-{proto,sdk} dependencies to the right branch (#301) [Corentin Henry]
- PB-159: remove weights from gRPC messages (#298) [Corentin Henry]
- PB-431 send participant state to influxdb (#300) [Robert Steiner]
- PB-434 separate metrics (#296) [Robert Steiner]
- PB-406 :snowflake: Configure mypy (#297) [Anastasiia Tymoshchuk]
- PB-428 send coordinator states (#292) [Robert Steiner]
- PB-425 split weight init from training (#295) [janpetschexain]
- PB-398 Round resumption in Coordinator (#285) [kwok]
- Merge pull request #294 from xainag/master. [Daniel Kravetz]
- Hotfix: PB-432 :pencil: :books: Update test badge and CI to reflect changes. [Daniel Kravetz]
- PB-417 Start new development cycle (#291) [Anastasiia Tymoshchuk, kwok]

## [0.5.0] - 2020-02-12

Fix minor issues, update documentation.

- PB-402 Add more logs (#281) [Robert Steiner]
- DO-76 :whale: non alpine image (#287) [Daniel Kravetz]
- PB-401 Add console renderer (#280) [Robert Steiner]
- DO-80 :ambulance: Update dev Dockerfile to build gRPC (#286) [Daniel Kravetz]
- DO-78 :sparkles: add grafana (#284) [Daniel Kravetz]
- DO-66 :sparkles: Add keycloak (#283) [Daniel Kravetz]
- PB-400 increment epoch base (#282) [janpetschexain]
- PB-397 Simplify write metrics function (#279) [Robert Steiner]
- PB-385 Fix xain-sdk test (#278) [Robert Steiner]
- PB-352 Add sdk config (#272) [Robert Steiner]
- Merge pull request #277 from xainag/master. [Daniel Kravetz]
- Hotfix: update ci. [Daniel Kravetz]
- DO-72 :art: Make CI name and feature consistent with other repos. [Daniel Kravetz]
- DO-47 :newspaper: Build test package on release branch. [Daniel Kravetz]
- PB-269: enable reading participants weights from S3 (#254) [Corentin Henry]
- PB-363 Start new development cycle (#271) [Anastasiia Tymoshchuk]
- PB-119 enable isort diff (#262) [janpetschexain]
- PB-363 :gem: Release v0.4.0. [Daniel Kravetz]
- DO-73 :green_heart: Disable continue_on_failure for CI jobs. Fix mypy. [Daniel Kravetz]

## [0.4.0] - 2020-02-04

Flatten model weights instead of using lists.
Fix minor issues, update documentation.

- PB-116: pin docutils version (#259) [Corentin Henry]
- PB-119 update isort config and calls (#260) [janpetschexain]
- PB-351 Store participant metrics (#244) [Robert Steiner]
- Adjust isort config (#258) [Robert Steiner]
- PB-366 flatten weights (#253) [janpetschexain]
- PB-379 Update black setup (#255) [Anastasiia Tymoshchuk]
- PB-387 simplify serve module (#251) [Corentin Henry]
- PB-104: make the tests fast again (#252) [Corentin Henry]
- PB-122: handle sigint properly (#250) [Corentin Henry]
- PB-383 write aggregated weights after each round (#246) [Corentin Henry]
- PB-104: Fix exception in monitor_hearbeats() (#248) [Corentin Henry]
- DO-57 Update docker-compose files for provisioning InfluxDB (#249) [Ricardo Saffi Marques]
- DO-59 Provision Redis 5.x for persisting states for the Coordinator (#247) [Ricardo Saffi Marques]
- PB-381: make the log level configurable (#243) [Corentin Henry]
- PB-382: cleanup storage (#245) [Corentin Henry]
- PB-380: split get_logger() (#242) [Corentin Henry]
- XP-332: grpc resource exhausted (#238) [Robert Steiner]
- XP-456: fix coordinator command (#241) [Corentin Henry]
- XP-485 Document revised state machine (#240) [kwok]
- XP-456: replace CLI argument with a config file (#221) [Corentin Henry]
- DO-48 :snowflake: :rocket: Build stable package on git tag with SemVer (#234) [Daniel Kravetz]
- XP-407 update documentation (#239) [janpetschexain]
- XP-406 remove numpy file cli (#237) [janpetschexain]
- XP-544 fix aggregate module (#235) [janpetschexain]
- DO-58: cache xain-fl dependencies in Docker (#232) [Corentin Henry]
- XP-479 Start training rounds from 0 (#226) [kwok]

## [0.3.0] - 2020-01-21

- XP-505 cleanup docstrings in xain_fl.coordinator (#228)
- XP-498 more generic shebangs (#229)
- XP-510 allow for zero epochs on cli (#227)
- XP-508 Replace circleci badge (#225)
- XP-505 docstrings cleanup (#224)
- XP-333 Replace numproto with xain-proto (#220)
- XP-499 Remove conftest, exclude tests folder (#223)
- XP-480 revise message names (#222)
- XP-436 Reinstate FINISHED heartbeat from Coordinator (#219)
- XP-308 store aggregated weights in S3 buckets (#215)
- XP-308 store aggregated weights in S3 buckets (#215)
- XP-422 ai metrics (#216)
- XP-119 Fix gRPC testing setup so that it can run on macOS (#217)
- XP-433 Fix docker headings (#218)
- Xp 373 add sdk as dependency in fl (#214)
- DO-49  Create initial buckets (#213)
- XP-424 Remove unused packages (#212)
- XP-271 fix pylint issues (#210)
- XP-374 Clean up docs (#211)
- DO-43  docker compose minio (#208)
- XP-384 remove unused files (#209)
- XP-357 make controller parametrisable (#201)
- XP 273 scripts cleanup (#206)
- XP-385 Fix docs badge (#204)
- XP-354 Remove proto files (#200)
- DO-17  Add Dockerfiles, dockerignore and docs (#202)
- XP-241 remove legacy participant and sdk dir (#199)
- XP-168 update setup.py (#191)
- XP-261 move tests to own dir (#197)
- XP-257 cleanup cproto dir (#198)
- XP-265 move benchmarks to separate repo (#193)
- XP-255 update codeowners and authors in setup (#195)
- XP-255 update codeowners and authors in setup (#195)
- XP-229 Update Readme.md (#189)
- XP-337 Clean up docs before generation (#188)
- XP-264 put coordinator as own package (#183)
- XP-272 Archive rust code (#186)
- Xp 238 add participant selection (#179)
- XP-229 Update readme (#185)
- XP-334 Add make docs into docs make file (#184)
- XP-291 harmonize docs styles (#181)
- XP-300 Update docs makefile (#180)
- XP-228 Update readme (#178)
- XP-248 use structlog (#173)
- XP-207 model framework agnostic (#166)
- XAIN-284 rename package name (#176)
- XP-251 Add ability to pass params per cmd args to coordinator (#174)
- XP-167 Add gitter badge (#171)
- Hotfix badge versions and style (#170)
- Integrate docs with readthedocs (#169)
- add pull request template (#168)

## [0.2.0] - 2019-12-02

### Changed

- Renamed package from xain to xain-fl

## [0.1.0] - 2019-09-25

The first public release of **XAIN**

### Added

- FedML implementation on well known
  [benchmarks](https://github.com/xaynetwork/xaynet/tree/v0.1.0/xain/benchmark) using
  a realistic deep learning model structure.

[Unreleased]: https://github.com/xaynetwork/xaynet/compare/v0.11.0...HEAD
[0.11.0]: https://github.com/xaynetwork/xaynet/compare/v0.10.0...v0.11.0
[0.10.0]: https://github.com/xaynetwork/xaynet/compare/v0.9.0...v0.10.0
[0.9.0]: https://github.com/xaynetwork/xaynet/compare/v0.8.0...v0.9.0
[0.8.0]: https://github.com/xaynetwork/xaynet/compare/v0.7.0...v0.8.0
[0.7.0]: https://github.com/xaynetwork/xaynet/compare/v0.6.0...v0.7.0
[0.6.0]: https://github.com/xaynetwork/xaynet/compare/v0.5.0...v0.6.0
[0.5.0]: https://github.com/xaynetwork/xaynet/compare/v0.4.0...v0.5.0
[0.4.0]: https://github.com/xaynetwork/xaynet/compare/v0.3.0...v0.4.0
[0.3.0]: https://github.com/xaynetwork/xaynet/compare/v0.2.0...v0.3.0
[0.2.1]: https://github.com/xaynetwork/xaynet/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/xaynetwork/xaynet/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/xaynetwork/xaynet/tree/v0.1.0
