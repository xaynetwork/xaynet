Network Architecture
====================

.. literalinclude:: ../protobuf/xain/grpc/coordinator.proto
    :language: proto

Introduction
------------


.. note::

    This section should contain an overview of the network architecture
    (coordinator, participant, grpc) mostly taken from the XP architecture
    hackmd.

**Participants:**
- Clients
- Coordinator

---

Clients need a bi-directional communication channel with the coordinator.

There is no client to client communication.

Coordinator just treats all clients equally and broadcasts it's messages to all
clients.

**Client -> Coordinator:**
- rendezvous
- heartbeat
- updates

**Coordinator -> Client:**
- rendezvous
- global model
- task to execute

---

**Flow:**

1. Instantiate a coordinator with the task to execute the number of clients
   required and the number of rounds to perform

.. code-block:: bash

    $ xain-coordinator fashion_mnist_100p_IID_balanced --clients=20 --rounds=50

2. Instantiate the clients with the coordinator address. If the coordinator is
   not reachable just periodically try to reconnect.

.. code-block:: bash

    $ xain-client ec2-198-51-100-1.compute-1.amazonaws.com --port=5000

3. Rendezvous
4. Once all necessary clients are connected, start the task:
    a. coordinator sends global model
    b. clients run the training
    c. clients send the updates (and any other relevant information)
5. Coordinator completes a round:
    a. Wait for all client updates
    b. aggregate
    c. Repeat 4 and 5
6. If any client gets disconnected during a round:
    a. discard the round
    b. wait for new clients to come back online until the necessary number of clients is met
    c. resume the task
7. Once all rounds are completed the coordinator can just exit


---

**Rendezvous:**

We can make it very simple in the beginning.

A client contacts the coordinator and the coordinator adds the client to its
list of clients. If the coordinator already has all the clients it needs tell
the client to try again later.

---

**Heartbeat:**

Clients periodically send an heartbeat so that the coordinator can detect failures.

If using gRPC _streams_ maybe this is already provided out of the box.

---

**Implementation:**

The main goal is to just add networking the the existing code.

The **client** implementation should be simple:
- create a gRPC channel
- run tasks sequentially

The **coordinator** implementation is a bit more complicated since it needs to
keep state about the current state of fedml training session and communicate
with multiple clients simultaneously.

I think that gRPC supports non-blocking or asynchronous calls so we may be able
to do this a single thread/process which would greatly simplify the
implementation. I'm sure there are examples out there on how to handle multiple
concurrent clients.

---

**Protobuf serialization:**

Regarding the serialization of some data structures like numpy arrays an
initial quick and dirty solution would to just marshal/pickle them into a byte
array and then just send them as raw bytes or a string using protobuf

---

**Messages between Coordinator and Participant:**

To see what is exchanged, see the `Coordinator` class e.g. the `train_local`
function

.. code-block:: python

    def train_local(
        p: Participant, theta: KerasWeights, epochs: int, epoch_base: int
    ) -> Tuple[Tuple[KerasWeights, int], KerasHistory, Metrics]:
        theta_update, history = p.train_round(theta, epochs, epoch_base)
        metrics = p.metrics()
    return theta_update, history, metrics

The participant needs from the coordinator

* `theta: KerasWeights` where `KerasWeights = List[ndarray]`
* `epochs: int`
* `epoch_base: int`

In return the participant sends back a tuple `theta_update, history` where

* `theta_update: Tuple[KerasWeights, int]`
* `history: KerasHistory` where `KerasHistory = Dict[str, List[float]]`

After a `train_round`, the coordinator also needs to invoke a participant's `metrics`. A `Metrics` gets sent back, where

* `Metrics = Tuple[int, VolumeByClass]`
* `VolumeByClass = List[int]`

We just need to keep in mind that with gRPC since the coordinator is the service all calls need to be initiated by the client. So we will need for the participant to poll the coordinator for the beginning of the round.
Also if the client needs to send some metrics at the end of a round maybe the last two messages could be combined. The participant would send the updates and metrics in the same call

---

**gRPC server side connection management**

In the context of the _xain_ project the coordinator is responsible for keeping
track of its connected participants that may be performing long running tasks.
In order to do that the coordinator needs to be capable to detect when a client
gets disconnected. This does not seem to be easy to achieve with gRPC (at least
not with the python implementation).

From a developers perspective gRPC behaves much like the request response pattern of a REST service. The server
doesn't typically care much about the clients and doesn't keep state between
calls. All calls are initiated by the client and the server simply servers the
request and forgets about the client.

This also means that there really isn't much support for long standing
connections. It's easy for a client to check the status of the connection to
the server but the opposite is not true.

gRPC does use mechanisms from the underlying HTTP and TCP transport layers but
these are internal details that aren't really exposed in the API. A developer
can override the default timeouts but it's not clear to me at this point the
effect they have. For more information check [using gRPC in
production](https://cs.mcgill.ca/~mxia3/2019/02/23/Using-gRPC-in-Production/).

It's also not clear how connections are handled internally. At least in the
python library when opening a channel no connection is made to the server. The
connection only happens when a method is actually called.

With the provided APIs from the server side we can only do any logic from
within a method call.

So far the only way I found that allow us to keep track of client connections
from the server side is to have the client calling a method that never returns.
From within that method the server can either:

**Add callback to get notified when an RPC call was terminated**

.. code-block:: python

    def rpc_terminated_callback(context):
        # do something with the context

    def SomeMethod(self, request, context):
        context.add_callback(lambda: rpc_terminated_callback(context))

        # rest of the method logic

The problem with this approach is that if we are blocking the method, the
method never really returns.

**Periodically check if a the rcp call is active**

.. code-block:: python

    def SomeMethod(self, request, context):
        while context.is_active():
            time.sleep(5)

        # if we reach this point the client terminated the call

The problem with this method is that we are wasting a thread **per** client
just to check the client connection.


**Right now I'm more inclined into implementing our own heartbeat solution**

.. code-block:: python

    def Hearthbeat(self, request, context):
        self.participants[context.peer()].expires = time.now() + KEEPALIVE_TIME
        return PingResponse()

    # in another thread periodically call/schedule
    def monitor_clients(self):
        for participant in self.participants:
            if participant.expires < time.now() + KEEPALIVE_TIMEOUT:
                # remove participant and perform any action necessary

This heartbeat can in the future be combined with any polling required from the
client side e.g. polling the coordinator for more tasks to perform.


Coordinator
-----------

.. note::

    This section should contain more specific details about the Coordinator
    (state machine, implementation details)

Supplementary thoughts about interaction between
coordinator :math:`C` and a participant :math:`P`, given the gRPC-based
setup described in *XP Network Architecture*. First let's consider the basic
lifecycle of state transitions in :math:`C`. Let :math:`N` be the number of
required participants.

.. mermaid::

    graph TB
    A( ) -->|startup| B(Registration)
    B -->|N Ps registered| C(Round open)
    C -->|all Ps trained| D(Round closed)
    D -->|more rounds| C
    D -->|no more rounds| E( )

Once :math:`C` starts up, it's in the **Registration** state, waiting for incoming
connections from participants looking to rendezvous. Once :math:`N` have been
registered, a number of these are selected for a training round. To simplify
for now, assume all :math:`N` will participate.

In the **Round open** state, :math:`C` starts to accept requests (from the
registered :math:`N`) to start training. Any further requests (from late
entrants) to rendezvous are told to "try again later". For any :math:`P` that
has started training, :math:`C` will also accept a subsequent request of it
having finished training.

Once all :math:`N` have finished training, :math:`C` closes the round. In the
**Round closed** state, :math:`C` collects together all the trained data and
aggregates them.  It either goes back to *Round open* if there are more rounds
to go, or else it exits.

So far we've only discussed the lifecycle of a *successful* interaction with
all participants i.e. without faults, dropouts, etc. The true picture (taking
into account of [fault tolerance](https://hackmd.io/gzGSJZ2xQTyERNjTpqguqg))
will be more complicated than above but this is still useful to give the basic
structure.


Participant
-----------

.. note::

    This section should contain more specific details about the Participant
    (state machine, implementation details, ...)

Now the state transitions of a participant :math:`P`.

.. mermaid::

    graph TB
    A( ) -->|startup| B(Discovery)
    B -->|rendezvous'd| C(Standby)
    C -->|selected| D(Ready)
    D -->|model received| E(Training)
    E -->|C ack updated model| C



Once :math:`P` starts up, it's in the **Discovery** state, looking for
:math:`C` to rendezvous with. When this is successful, it's in **Standby**
essentially until :math:`C` signals it's been selected to start training (in
practice, :math:`P` would need to poll :math:`C` for this state change).

In the next state, :math:`P` is **Ready** (to start training). It indicates this to
:math:`C`, waiting for a model to train on in response. :math:`P` works on this in the
**Training** state and indicates to :math:`C` when it's complete, sending along the
updated model. :math:`P` goes back to Standby when :math:`C` acknowledges receipt.

Again, this only shows the state transitions of the "success" case. The more
refined picture will take account of various fault scenarios. For example, if
:math:`C` decides to cancel the round, :math:`P` goes back to Standby (possibly cancelling
or discarding any training already started). Or worse, if the connection with
:math:`C` is lost, :math:`P` goes back to Discovery.


The following is a more refined picture of the :math:`P` state machine. It focuses on
the state transitions in response to heartbeat messages described above, and is
also able to handle *selection*.

.. mermaid::

graph TB
A( ) -.->|rendezvous| B(Wait for Selection)
B -->|ROUND i| C(Training i)
D -->|ROUND i+1| C
C -.->|trained| D(Post-training i)
D -->|FINISHED| E(Done)
D -->|STANDBY| B



After a successful rendezvous, :math:`P` is in **Wait for Selection**. :math:`P` waits in
this state as long as it keeps receiving `STANDBY` heartbeats. At some round
$i$, :math:`C` may select :math:`P` for the round by responding with a `ROUND` $i$
heartbeat. At this point, :math:`P` moves to **Training** where the above sequence of
training messages (`StartTraining` $\rightarrow \theta \rightarrow \theta'
\rightarrow$ `EndTraining`) occur. Having received the `EndTraining` reply from
:math:`C`, :math:`P` makes an "internal" transition to **Post-training** where it waits
until the start of the next round. If it has been selected again, it will
observe `ROUND` $i+1$. If not, it observes `STANDBY`. Alternatively, if round
$i$ was the last, it instead sees `FINISHED` and :math:`P` is **Done**. Note that
`FINISHED` can also be observed from **Wait for Selection** but the transition
from there to **Done** is omitted in the diagram just for sake of clarity.
