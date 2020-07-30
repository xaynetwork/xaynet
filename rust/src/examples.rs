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
min_sum_time = 5
min_update_time = 10
sum = 0.4
update = 0.9
expected_participants = 20

[mask]
group_type = "Prime"
data_type = "F32"
bound_type = "B0"
model_type = "M3"

[model]
size = 4
```

The actual files contain more settings than this, but we mention just the
selection above because they will be the most relevant for this guide.

## Settings

Going from the top, the [`ApiSettings`] include [`bind_address`], which is the
address the coordinator should listen on for requests from participants. This
address should be known to all participants.

The [`PetSettings`] specify various parameters of the PET protocol.

- The most
important are [`sum`] and [`update`], which are the probabilities assigned to
the selection of sum and update participants, respectively (note that if a
participant is selected for both roles, the *sum* role takes precedence).

- The settings [`min_sum_count`] and [`min_update_count`] specify, respectively,
the minimum number of `sum`/`sum2` and `update` messages the coordinator should
accept. By default, they are set to the theoretical minimum in order for the
protocol to function correctly.

- To complement, the settings [`min_sum_time`] and
[`min_update_time`] specify, respectively, the minimum amount of time (in
seconds) the coordinator should wait for `sum`/`sum2` and `update` messages. To
allow for more messages to be processed, increase these times.

- [`expected_participants`] should be set to the expected number of participants
for the session.

The [`MaskSettings`] determines the masking configuration, consisting of the
group type, data type, bound type and model type. The [`ModelSettings`] specify
the size of the model used. Both of these settings should be decided in advance
with participants, and agreed upon by both.

## Running

The coordinator can be run as follows:

```ignore
$ git clone git://github.com/xaynetwork/xaynet
$ cd xaynet/rust
$ cargo run --bin coordinator -- -c ../configs/config.toml
```

# Connecting with Participants

The below shows an example of how participants can be created to connect to the
coordinator running as instructed above. It is a slightly adapted version of
the `test-drive-net` example, for illustrative purposes.

```no_run
use tokio::signal;
use xaynet::{
    client::{Client, ClientError},
    mask::{FromPrimitives, Model},
};

#[tokio::main]
async fn main() -> Result<(), ClientError> {
    let len = 4_usize;
    let model = Model::from_primitives(vec![0; len].into_iter()).unwrap();

    let mut clients = Vec::with_capacity(20_usize);
    for id in 0..clients.len() {
        let mut client = Client::new_with_addr(1, id, "http://127.0.0.1:8081")?;
        client.local_model = Some(model.clone());
        let join_hdl = tokio::spawn(async move {
            tokio::select! {
                _ = signal::ctrl_c() => {}
                result = client.start() => {
                    error!("{:?}", result);
                }
            }
        });
        clients.push(join_hdl);
    }

    for client in clients {
        let _ = client.await;
    }

    Ok(())
}
```

As this example is meant solely as a demonstration of the operation of
the PET protocol, it does not perform any training as such (see other
forthcoming examples for that). Instead, a "dummy" zero-model is used throughout.
Nevertheless, note that its length and contents are (respectively) required to
match the size and mask configuration expected by the coordinator.

The example creates the expected number of participants (called [`Client`]s
here), connecting to the address where the coordinator should be listening. The
participants are then started, and run continuously until stopped (with CTRL +
C).

## Running

The actual `test-drive-net` example is a tidier version of the above, where the
hard-coded numbers are made configurable. To run:

```no_run
$ cargo run --example test-drive-net -- -l 4 -n 20 -u http://127.0.0.1:8081
```
*/
