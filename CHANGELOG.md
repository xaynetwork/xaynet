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
  [benchmarks](https://github.com/xainag/xain-fl/tree/master/benchmarks/benchmark) using
  a realistic deep learning model structure.

[unreleased]: https://github.com/xainag/xain-fl/pulls?utf8=%E2%9C%93&q=merged%3A%3E2019-09-25+
[0.1.0]: https://github.com/xainag/xain-fl/pulls?utf8=%E2%9C%93&q=merged%3A%3C%3D2019-09-25+
