# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a
Changelog](https://keepachangelog.com/en/1.0.0/), and this project adheres to
the [Python form of Semantic
Versioning](https://www.python.org/dev/peps/pep-0440/).

For reference, the possible headings are:

- `Added` for new features.
- `Changed` for changes in existing functionality.
- `Deprecated` for soon-to-be removed features.
- `Removed` for now removed features.
- `Fixed` for any bug fixes.
- `Security` in case of vulnerabilities.
- `External Contributors` to list all external contributors.
- `Notes` for notes regarding this particular release.

## [Unreleased]

## [0.7.0] - 2020-03-25

- PB-584 Update the manifest Cargo.toml with description, license, keywords, repository URL and project homepage (#343) [Ricardo Saffi Marques]
- PB-584 Add LICENSE file at the root of the repository prior to release v0.7.0 (#342) [Ricardo Saffi Marques]
- Update Cargo.lock for release. [Ricardo Saffi Marques]
- Update versions and authors (#335) [Corentin Henry]
- Remove the rustfmt config file (#341) [Corentin Henry]
- Remove nix files (#337) [Corentin Henry]
- Remove caddy (#338) [Robert Steiner]
- Nix-shell: install rust-analyzer (#336) [Corentin Henry]
- Re-use the CHANGELOG file from the legacy codebase. [little-dude]
- Rewrite xain-fl in Rust. [little-dude]
- Merge pull request #64 from xainag/optional-metric-store. [Corentin Henry]
- In docker, compile with `influx_metrics` [little-dude]
- Silence "unused variable" warning from rustc. [little-dude]
- Add CI jobs for the influx_metrics feature. [little-dude]
- Introduce the influx_metrics feature. [little-dude]
- Disable metrics when running locally. [little-dude]
- Make the metric store optional. [little-dude]
- Move the metric store to xain_fl::common. [little-dude]
- Merge pull request #68 from xainag/ci-build-all-features. [Corentin Henry]
- Fix build for the telemetry feature. [little-dude]
- Ci: add matrix to tests various --features flags. [little-dude]
- Merge pull request #67 from xainag/example-fixes. [Corentin Henry]
- Sdk: participant should exit when a heartbeat fails. [little-dude]
- Sdk: crash instead of calling sys.exit when participant errors out. [little-dude]
- Fix exit condition in dummy example. [little-dude]
- Merge pull request #65 from xainag/less-verbose-logging. [Corentin Henry]
- Make logging more finely configurable. [little-dude]
- Merge pull request #66 from xainag/fix-crash. [Corentin Henry]
- Fix crash after training finishes. [little-dude]
- Merge pull request #63 from xainag/misc. [Corentin Henry]
- Log an error when a heartbeat is rejected. [little-dude]
- Use more reasonable values in docker-release-aggregator.toml. [little-dude]
- Sdk: make the heartbeat frequency configurable. [little-dude]
- Simplify match statement. [little-dude]
- Nix: add some cargo tools. [little-dude]
- Merge pull request #61 from xainag/refactor. [Corentin Henry]
- Refactor: simplify RPC code by using the ServiceHandle. [little-dude]
- Merge pull request #52 from xainag/PB-490-protocol-tests. [Corentin Henry]
- Fix protocol test: by default expect two rounds. [little-dude]
- Document the protocol tests. [little-dude]
- Remove ignore. [Robert Steiner]
- Remove unused function. [Robert Steiner]
- Fix typo. [Robert Steiner]
- Clean up. [Robert Steiner]
- Add full training test case. [Robert Steiner]
- Clean up tests. [Robert Steiner]
- Add end_training and end_aggregation tests. [Robert Steiner]
- Add endtraining test. [Robert Steiner]
- Remove comments. [Robert Steiner]
- Add start training tests. [Robert Steiner]
- Add heartbeat test. [Robert Steiner]
- Fix tests. [Robert Steiner]
- Add heartbeat tests. [Robert Steiner]
- PB-490 Add protocol tests. [Robert Steiner]
- Merge pull request #59 from xainag/opentelemetry. [Corentin Henry]
- Integrate with opentelemetry and jaeger. [little-dude]
- Get rid of log and env_logger dependencies. [little-dude]
- Small refactoring. [little-dude]
- Configure a custom Subscriber for the aggregator. [little-dude]
- Split AggregatorService::poll_rpc_requests() [little-dude]
- Implement Display for RPC requests. [little-dude]
- Switch to `tracing` for logging. [little-dude]
- Update README. [little-dude]
- Merge pull request #56 from xainag/PB-491-test-python-ffi. [Corentin Henry]
- Add reset, get_global_weights and aggregate tests. [Robert Steiner]
- Add add_weights and aggregate test. [Robert Steiner]
- Add python setup in rust-test. [Robert Steiner]
- Add more tests. [Robert Steiner]
- Add load test. [Robert Steiner]
- Update README. [little-dude]
- Add instructions to run the examples. [little-dude]
- Remove more dependencies. [little-dude]
- Remove unused dependency. [little-dude]
- Bump dependencies. [little-dude]
- Merge pull request #54 from xainag/simplify_rpc_client. [Corentin Henry]
- Simplify the RPC implementation. [little-dude]
- Install docker-compose. [little-dude]
- Merge pull request #53 from xainag/error-handling. [Corentin Henry]
- Aggregator: properly handle CTRL+C. [little-dude]
- Aggregator: when a task finishes cancel the other ones. [little-dude]
- Rename channels to reflect whether they are senders or receivers. [little-dude]
- Improve error handling in the py_aggregator module. [little-dude]
- Add anyhow and thiserror dependencies for error handling. [little-dude]
- Merge pull request #49 from xainag/fix-clippy-warnings. [Corentin Henry]
- Update ci. [Robert Debug]
- Fix clippy warnings. [Robert Debug]
- Deny warnings when compiling rust code. [little-dude]
- Do not run CI for PRs. [little-dude]
- Configure python CI (#50) [Robert Steiner]
- Remove dummy aggregator. [little-dude]
- Cleanup the keras_house_prices example. [little-dude]
- Add vscode workspace settings to gitignore. [Robert Debug]
- Clean up write metrics. [Robert Steiner]
- Merge pull request #37 from xainag/benchmarks. [Corentin Henry]
- Delete dummy tensorflow example. [little-dude]
- Cleanup the keras benchmark. [little-dude]
- Optimize dummy example. [little-dude]
- Fix data_handlers. [little-dude]
- Beef up the .dockerignore to speed up docker. [little-dude]
- Port keras example from the benchmarks repo. [little-dude]
- Remove junk files. [little-dude]
- Beef up the .dockerignore to speed up docker. [little-dude]
- Merge pull request #44 from xainag/add_metric_queue. [Corentin Henry]
- Remove tests. [Robert Steiner]
- Fix config. [Robert Steiner]
- Add metric queue. [Robert Steiner]
- Merge pull request #41 from xainag/valgrind. [Robert Steiner]
- Remove valgrind in dev. [Robert Steiner]
- Clean up. [Robert Steiner]
- Fix rebase. [Robert Steiner]
- Add valgrind. [Robert Debug]
- Merge pull request #43 from xainag/memory-leak. [Corentin Henry]
- Fix python memory leak. [little-dude]
- Fix warning about unused import. [little-dude]
- Configure logging for the weighted average python aggregator. [little-dude]
- In the tf.py example just pretend to train. [little-dude]
- Make participant logging less noisy. [little-dude]
- Merge pull request #38 from xainag/add_rendez_vous_tests. [Robert Steiner]
- Fmt. [Robert Steiner]
- Clean up tests. [Robert Steiner]
- Add rendez_vous tests. [Robert Steiner]
- Rustfmt. [little-dude]
- Fix CI. [little-dude]
- Set working directory in CI. [little-dude]
- Fix typo in github workflow file. [little-dude]
- Add CI for rust. [little-dude]
- Document how to run release builds. [little-dude]
- Update images. [little-dude]
- Add top level .gitignore. [little-dude]
- Move the rust code into its own directory. [little-dude]
- Add instructions for profiling. [little-dude]
- TEST CONFIG. [little-dude]
- Add instructions to run the tf example. [little-dude]
- Sdk: handle start training rejections. [little-dude]
- Fix shebang for nixos. [little-dude]
- Tweak the log levels. [little-dude]
- Use different tags for debug/release docker images. [little-dude]
- Require more participants for release builds. [little-dude]
- Docker: add release build and move configs out of docker dir. [little-dude]
- Fix metric store tests. [Robert Steiner]
- Merge pull request #32 from xainag/add-more-metrics. [Robert Steiner]
- Delete old dashboard. [Robert Steiner]
- Replace String with &'static str. [Robert Steiner]
- Add more metrics. [Robert Steiner]
- Add run participant script. [Robert Steiner]
- Use real aggregator in docker. [little-dude]
- Log why a start training request is rejected. [little-dude]
- Train on smaller models (1M) [little-dude]
- Fix py_aggregator memory leak. [little-dude]
- Fix Dockerfile. [little-dude]
- More profiling tools. [little-dude]
- Nix: add gperftools dependency to debug aggregator memory leak. [little-dude]
- Merge pull request #33 from xainag/aggregators. [Corentin Henry]
- Add tensorflow example. [little-dude]
- Make the sdk more generic. [little-dude]
- Formatting and pylint fixes. [little-dude]
- Add real aggregators. [little-dude]
- Merge pull request #34 from xainag/docker-stats. [Robert Steiner]
- Update caddyfile. [Robert Steiner]
- Remove alertmanager. [Robert Steiner]
- Remove alert manager. [Robert Steiner]
- Add docker stats. [Robert Debug]
- Repo cleanup. [little-dude]
- Make the python aggregator configurable. [little-dude]
- Fix aggregator port. [Robert Steiner]
- Add numpy. [Robert Steiner]
- Merge pull request #30 from xainag/collect_metrics. [Robert Steiner]
- Remove clone impl. [Robert Steiner]
- Collect Metrics. [Robert Steiner]
- Fix ports in config files. [little-dude]
- Sdk: various fixes. [little-dude]
- Some more logging. [little-dude]
- Wait for end aggregation message from the aggregator to enter finish state. [little-dude]
- Don't panic upon un-expected end aggregation message from the aggregator. [little-dude]
- Handle protocol events after polling the aggregation future. [little-dude]
- Move DummyParticipant out of sdk package. [little-dude]
- Working full round. [little-dude]
- Working upload. [little-dude]
- Python run black. [little-dude]
- Some renaming. [little-dude]
- Nix: fix build. [little-dude]
- Basic upload handling. [little-dude]
- Delete outdated examples. [little-dude]
- Implement downloading weights. [little-dude]
- Merge pull request #16 from xainag/metrics_store. [Robert Steiner]
- Applied review changes. [Robert Steiner]
- Add metrics store tests. [Robert Steiner]
- Add metric store. [Robert Debug]
- WIP. [Robert Debug]
- Call poll. [Robert Debug]
- Add metric store. [Robert Debug]
- Fix RequestStream and RequestReceiver. [little-dude]
- Merge pull request #25 from xainag/expose-rest-api. [Robert Steiner]
- Fix docker build. [Robert Steiner]
- Expose REST API. [Robert Debug]
- Nix: move back to lorri. [little-dude]
- Client: implement start_training. [little-dude]
- Show milliseconds in log timestamps. [little-dude]
- Remove docstring. [little-dude]
- Fix build. [little-dude]
- Start client implementation + minor fixes. [little-dude]
- Quick & dirty json serialization. [little-dude]
- Fix undeclared module. [Robert Debug]
- Improve settings error handling. [Robert Debug]
- Better logging for the python client. [little-dude]
- Fix coordinator api paths. [little-dude]
- Copy Participant abstract class from xain-sdk. [little-dude]
- Rough http client. [little-dude]
- Make timeout configurable. [little-dude]
- Update config files. [little-dude]
- Nix: do not re-install the python client in the shell hook. [little-dude]
- More logging in the protocol. [little-dude]
- Move settings to their own module. [little-dude]
- Merge pull request #23 from xainag/error-handling-improvement. [Robert Steiner]
- Replace match with ? [Robert Steiner]
- Small improvements. [Robert Steiner]
- Create package for a python client. [little-dude]
- Start the HTTP server in both service. [little-dude]
- Fix path in .dockerignore. [little-dude]
- Create a docker directory. [little-dude]
- Use specific config files for docker. [little-dude]
- Merge pull request #22 from xainag/docker-bin-config. [Corentin Henry]
- Add bin configs in docker-compose. [Robert Steiner]
- Fix coordinator config file. [little-dude]
- Add config files. [little-dude]
- Merge pull request #19 from xainag/docker. [Corentin Henry]
- Update docker files. [Robert Debug]
- Update readme. [Robert Steiner]
- Update bin section Cargo (used for cargo vendor) [Robert Steiner]
- Add new line. [Robert Steiner]
- Small improvements. [Robert Steiner]
- Add dockerignore. [Robert Steiner]
- Add docker file. [Robert Steiner]
- Implement http layer for the coordinator. [little-dude]
- Bump dependencies. [little-dude]
- Implement http api for the aggregator. [little-dude]
- Merge pull request #20 from xainag/repo-setup. [Corentin Henry]
- Add new line. [Robert Steiner]
- Update gitignore, add rust toolchain. [Robert Steiner]
- Implement AggregatorServiceHandle. [little-dude]
- Small cleanup. [little-dude]
- Very rough implementation of aggregation. [little-dude]
- Rename rpc aggregator method: reset->aggregate. [little-dude]
- Add Protocol.waiting_for_aggregation. [little-dude]
- Add logging. [little-dude]
- Use stubborn-io. [little-dude]
- Split the aggregator out of the coordinator. [little-dude]
- Dummy aggregator::main() implementation. [little-dude]
- Start implementing the AggregatorService future. [little-dude]
- Document the RPC module. [little-dude]
- Implement spawn_rpc() [little-dude]
- Documentation, comments and logging for AggregatorService. [little-dude]
- More rpc work. [little-dude]
- Add commented out code. [little-dude]
- Rework crate structure. [little-dude]
- Update README. [little-dude]
- Implement AggregatorTarpcServiceHandle. [little-dude]
- Add diagram of envisioned architecture. [little-dude]
- More py_aggregator work. [little-dude]
- Move PyAggregator to crate::aggregator. [little-dude]
- Start playing around with pyo3. [little-dude]
- Bump dependencies. [little-dude]
- Split examples and binaries. [little-dude]
- Clippy + rustfmt. [little-dude]
- First steps with the aggregator service: rpc. [little-dude]
- Update sequence diagram. [little-dude]
- Add a "common" module. [little-dude]
- More logs and use reasonable values for example. [little-dude]
- Require T: Debug. [little-dude]
- Add readme. [little-dude]
- Quick & dirty client implementation. [little-dude]
- Start working on an example and fixing issues. [little-dude]
- Rename state_machine into protocol + various cleanups. [little-dude]
- Add sanity checks for counters. [little-dude]
- Add new() methods. [little-dude]
- Simplify state machine. [little-dude]
- Implement start training and end training. [little-dude]
- Get rid of the StateMachinEventHandler trait. [little-dude]
- Implement selection. [little-dude]
- Add coordinator handle. [little-dude]
- Remove unused imports. [little-dude]
- State machine cleanups. [little-dude]
- Impl StateMachineHandler for CoordinatorService. [little-dude]
- Get rid of the Client wrapper. [little-dude]
- Initial commit. [little-dude]
- Archive the legacy code. [little-dude]

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
  [benchmarks](https://github.com/xainag/xain-fl/tree/master/benchmarks/benchmark) using
  a realistic deep learning model structure.

[unreleased]: https://github.com/xainag/xain-fl/pulls?utf8=%E2%9C%93&q=merged%3A%3E2019-09-25+
[0.1.0]: https://github.com/xainag/xain-fl/pulls?utf8=%E2%9C%93&q=merged%3A%3C%3D2019-09-25+
