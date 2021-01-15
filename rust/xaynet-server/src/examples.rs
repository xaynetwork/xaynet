/*!
A guide to getting started with the XayNet examples.

# Examples

The XayNet examples code can be found under the `rust/examples` directory of the
[`xaynet`](https://github.com/xaynetwork/xaynet/) repository.

This Getting Started guide will cover only the general ideas around usage of the
examples. Also see the source code of the individual examples themselves, which
have plenty of comments.

Running an example typically requires having a *coordinator* already running,
which is the core component of XayNet.

# Federated Learning

A federated learning session over XayNet consists of two kinds of parties - a
*coordinator* and (multiple) *participants*. The two parties engage in a
protocol (called PET) over a series of rounds. The over-simplified idea is that
in each round:

1. The coordinator makes available a *global* model, from which selected
participants will train model updates (or, *local* models) to be sent back to
the coordinator.

2. As a round progresses, the coordinator aggregates these updates
into a new global model.

From this description, it might appear that individual local models are plainly
visible to the coordinator. What if sensitive data could be extracted from them?
Would this not be a violation of participants' data privacy?

In fact, a key point about this process is that the updates are **not** sent in
the plain! Rather, they are sent encrypted (or *masked*) so that the coordinator
(and by extension, XayNet) learns almost nothing about the individual updates.
Yet, it is nevertheless able to aggregate them in such a way that the resulting
global model is unmasked.

This is essentially what is meant by federated learning that is
*privacy-preserving*, and is a key feature enabled by the PET protocol.

## PET Protocol

It is worth describing the protocol very briefly here, if only to better
understand some of the configuration settings we will meet later. It is helpful
to think of each round being divided up into several contiguous phases:

**Start.** At the start of a round, the coordinator generates a collection of random *round
parameters* for all participants. From these parameters, each participant is
able to determine whether it is selected for the round and if so, which of the
two roles it is:

- *update* participants.

- *sum* participants.

**Sum.** In the Sum phase, sum participants send `sum` messages to the
coordinator (the details of which are not so important here, but vital for
computing `sum2` messages later).

**Update.** In the Update phase, each update participant obtains the global
model from the coordinator, trains a local model from it, masks it, and sends it
to the coordinator in the form of `update` messages. The coordinator will
internally aggregate these (masked) local models.

**Sum2.** In the Sum2 phase, sum participants compute the sum of masks over all
the local models, and sends it to the coordinator in the form of `sum2` messages.

Equipped with the sum of masks, the coordinator is able to *unmask* the
aggregated global model, for the next round.

This short description of the protocol skips over many details, but is
sufficient for the purposes of this guide. For a much more complete
specification, see the [white paper](https://www.xain.io/assets/XAIN-Whitepaper.pdf).

# Coordinator

The coordinator is configurable via various settings. The project contains
various ready-made configuration files that can be used, found under the
`configs` directory of the repository. Typically they look something like
the following (in TOML format):

```toml
[api]
bind_address = "127.0.0.1:8081"

[pet]
min_sum_count = 1
min_update_count = 3
min_sum2_count = 1
max_sum_count = 100
max_update_count = 10000
max_sum2_count = 100
min_sum_time = 5
min_update_time = 10
min_sum2_time = 5
max_sum_time = 3600
max_update_time = 3600
max_sum2_time = 3600
sum = 0.1
update = 0.9

[mask]
group_type = "Prime"
data_type = "F32"
bound_type = "B0"
model_type = "M3"

[model]
length = 4
```

The actual files contain more settings than this, but we mention just the
selection above because they will be the most relevant for this guide.

## Settings

Going from the top, the [`ApiSettings`] include the
address the coordinator should listen on for requests from participants. This
address should be known to all participants. Optionally, it also contains configurations for TLS
server and client authentication.

The [`PetSettings`] specify various parameters of the PET protocol:

- The most important are [`sum`] and [`update`], which are the probabilities assigned to the
selection of sum and update participants, respectively (note that if a participant is selected for
both roles, the *sum* role takes precedence).

- The settings [`min_sum_count`], [`min_update_count`] and [`min_sum2_count`] specify, respectively,
the minimum number of `sum`, `update` and `sum2` messages the coordinator should accept. Similarly,
the [`max_sum_count`], [`max_update_count`] and [`max_sum2_count`] specify the maximum number of
`sum`, `update` and `sum2` messages the coordinator should accept.

- To complement, the settings [`min_sum_time`], [`min_update_time`] and [`min_sum2_time`] specify,
respectively, the minimum amount of time (in seconds) the coordinator should wait for `sum`,
`update` and `sum2` messages. To allow for more messages to be processed, increase these times.
Similarly, the [`max_sum_time`], [`max_update_time`] and [`max_sum2_time`] specify the maximum
amount of time (in seconds) the coordinator should wait for `sum`, `update` and `sum2` messages.

The [`MaskSettings`] determines the masking configuration, consisting of the
group type, data type, bound type and model type. The [`ModelSettings`] specify
the length of the model used. Both of these settings should be decided in advance
with participants, and agreed upon by both.

## Running

The coordinator can be run as follows:

```text
$ git clone git://github.com/xaynetwork/xaynet
$ cd xaynet/rust
$ cargo run --bin coordinator -- -c ../configs/config.toml
```


## Running participants

You can run the example from the xaynet repository:

```text
$ git clone https://github.com/xaynetwork/xaynet
$ cf xaynet/rust/examples
$ RUST_LOG=info cargo run --example test-drive -- -n 10
```

[`ApiSettings`]: crate::settings::ApiSettings
[`PetSettings`]: crate::settings::PetSettings
[`sum`]: crate::settings::PetSettings::sum
[`update`]: crate::settings::PetSettings::update
[`min_sum_count`]: crate::settings::PetSettings::min_sum_count
[`min_update_count`]: crate::settings::PetSettings::min_update_count
[`min_sum2_count`]: crate::settings::PetSettings::min_sum2_count
[`max_sum_count`]: crate::settings::PetSettings::max_sum_count
[`max_update_count`]: crate::settings::PetSettings::max_update_count
[`max_sum2_count`]: crate::settings::PetSettings::max_sum2_count
[`min_sum_time`]: crate::settings::PetSettings::min_sum_time
[`min_update_time`]: crate::settings::PetSettings::min_update_time
[`min_sum2_time`]: crate::settings::PetSettings::min_sum2_time
[`max_sum_time`]: crate::settings::PetSettings::max_sum_time
[`max_update_time`]: crate::settings::PetSettings::max_update_time
[`max_sum2_time`]: crate::settings::PetSettings::max_sum2_time
[`MaskSettings`]: crate::settings::MaskSettings
[`ModelSettings`]: crate::settings::ModelSettings
*/
