Network Architecture
====================

Introduction
------------


Federated Machine Learning is a distributed machine learning approach. In its
simplest form it is composed of one *Coordinator* and a set of *Participants*.

The **Coordinator** is responsible for keeping any state required for the machine
learning task, orchestrate the machine learning task across a set of
*Participants*, and perform the *Aggregation* of the individual updates
returned by the *Participants*.

The **Participants** are mostly stateless process that receive from the
*Coordinator* a global model and the machine learning task to execute. Once
they finish executing the machine learning task they return to the
*Coordinator* the updated model.


Federated Machine Learning Flow
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

1. Instantiate a *Coordinator* with the task to execute the number of clients
   required and the number of rounds to perform (and any other relevant information)

.. code-block:: bash

    $ xain-coordinator fashion_mnist_100p_IID_balanced --clients=20 --rounds=50

2. Instantiate the *Participants* with the *Coordinator* address. If the *Coordinator* is
   not reachable just periodically try to reconnect.

.. code-block:: bash

    $ xain-client ec2-198-51-100-1.compute-1.amazonaws.com --port=5000

3. Rendezvous
4. Once all necessary *Participants* are connected, start a round:
    a. *Coordinator* sends global model
    b. *Participants* run the training
    c. *Participants* send the updates (and any other relevant information)
5. *Coordinator* completes a round:
    a. Wait for all *Participants* updates
    b. Run the *Aggregation* on the individual updates
    c. Repeat 4 and 5
6. If any *Participant* gets disconnected during a round:
    a. Wait for new *Participants* to come back online until the necessary number of clients is met
    b. Resume the task
7. Once all rounds are completed the *Coordinator* can just exit


Coordinator
-----------

This section discusses the design and implementation details of the
*Coordinator*.

**Requirements and Assumptions:**

* We need a bi-direction communication channel between *Participants* and *Coordinator*.
* There is no need for a *Participant* to *Pariticipant* communication.
* The *Pariticipants* run on the clients infrastructure. They should have low operation overhead.
* We need to be agnostic of the machine learning framework used by the clients.
* Keep in mind that the *Coordinator* may need to handle a large number of *Participants*.

**Features that need to be provided by the Coordinator:**

* Ability for *Participants* to register with it.
* Ability for *Participants* to retrieve the global model.
* Ability for *Participants* to submit their updated model.
* Ability for the *Coordinator* to orchestrate the training.
* Ability to keep track of the liveness of *Participants*.

gRPC and Protobuf
^^^^^^^^^^^^^^^^^

For the networking implementation we are using gRPC and for the data
serialization we are using protobuf.

The *Coordinator* is implemented as a gRPC service and provides 3 main methods.

A **Rendezvous** method that allows *Participants* to register with a
*Coordinator*. When handling this call the *Coordinator* may create some state
about the *Participant* in order to keep track of what the *Participant* is
doing.

A **StartTraining** method that allows *Participants* to get the current global
model as well as signaling their intent to participate in a given round.

A **EndTraining** method that allows *Participants* to submit their updated
models after they finished their training task.


In order to remain agnostic to the machine learning framework *Participants*
and *Coordinator* exchange models on the form of numpy arrays. How models are
converted from a particular machine learning framework model into numpy arrays
are outside the scope of this document. We do provide the `Numproto
<https://github.com/xainag/numproto>`_ python package that performs
serialization and deserialization of numpy arrays into and from protobuf.


gRPC Implementation Challenges
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

**1. Keeping track of Participants liveness**

The coordinator is responsible for keeping track of its connected participants
that may be performing long running tasks.  In order to do that the coordinator
needs to be capable to detect when a client gets disconnected. This does not
seem to be easy to achieve with gRPC (at least not with the python
implementation).

From a developers perspective gRPC behaves much like the request response
pattern of a REST service. The server doesn't typically care much about the
clients and doesn't keep state between calls. All calls are initiated by the
client and the server simply serves the request and forgets about the client.

This also means that there really isn't much support for long standing
connections. It's easy for a client to check the status of the connection to
the server but the opposite is not true.

gRPC does use mechanisms from the underlying HTTP and TCP transport layers but
these are internal details that aren't really exposed in the API. A developer
can override the default timeouts but it's not clear from the available
documentation the effect they have. For more information check [using gRPC in
production](https://cs.mcgill.ca/~mxia3/2019/02/23/Using-gRPC-in-Production/).

*Server-side timeouts configuration:*

.. code-block:: python

    server = grpc.server(
        futures.ThreadPoolExecutor(max_workers=10),
        options=(
            ('grpc.keepalive_time_ms', 10000),
            # send keepalive ping every 10 second, default is 2 hours
            ('grpc.keepalive_timeout_ms', 5000),
            # keepalive ping time out after 5 seconds, default is 20 seoncds
            ('grpc.keepalive_permit_without_calls', True),
            # allow keepalive pings when there's no gRPC calls
            ('grpc.http2.max_pings_without_data', 0),
            # allow unlimited amount of keepalive pings without data
            ('grpc.http2.min_time_between_pings_ms', 10000),
            # allow grpc pings from client every 10 seconds
            ('grpc.http2.min_ping_interval_without_data_ms',  5000),
            # allow grpc pings from client without data every 5 seconds
        )
    )

*Client-side timeouts configuration:*

.. code-block:: python

    stub = Stub(
          'localhost:50051', :this_channel_is_insecure,
          channel_args: {
            'grpc.keepalive_time_ms': 10000,
            'grpc.keepalive_timeout_ms': 5000,
            'grpc.keepalive_permit_without_calls': true,
            'grpc.http2.max_pings_without_data': 0,
            'grpc.http2.min_time_between_pings_ms':10000,
            'grpc.http2.min_ping_interval_without_data_ms': 5000,
          }
      )

It's also not clear how connections are handled internally. At least in the
python library when opening a channel no connection seems to be made to the
server. The connection only happens when a method is actually called.

With the provided APIs from the server side we can only do any logic from
within a method call.

From the python gRPC documentation there seems to be two ways that allow us to
keep track of client connections from the server side is to have the client
calling a method that never returns.  From within that method the server can
either:

*Add callback to get notified when an RPC call was terminated:*

.. code-block:: python

    def rpc_terminated_callback(context):
        # do something with the context

    def SomeMethod(self, request, context):
        context.add_callback(lambda: rpc_terminated_callback(context))

        # rest of the method logic

*Periodically check if a the rcp call is active:*

.. code-block:: python

    def SomeMethod(self, request, context):
        while context.is_active():
            time.sleep(5)

        # if we reach this point the client terminated the call

The problem with these approaches is that we need to block the gRPC method call
in order to keep track of the connection status. There are two problems with
these long standing connections: we are wasting server resources to do nothing,
and we need to deal with the underlying gRPC connection timeouts as described
above.

Ultimately we decided to just implement ourselves a simple heartbeat solution.
The *Participants* periodically send a heartbeat to the *Coordinator*. If the
*Coordinator* doesn't hear from a *Participant* after a pre-defined timeout if
just considers the *Participant* to be down and removes the *Participant* from
it's participant list.

*Heartbeat:*

.. code-block:: python

    def Hearthbeat(self, request, context):
        self.participants[context.peer()].expires = time.now() + KEEPALIVE_TIME
        return PingResponse()

    # in another thread periodically call/schedule
    def monitor_clients(self):
        for participant in self.participants:
            if participant.expires < time.now() + KEEPALIVE_TIMEOUT:
                # remove participant and perform any action necessary


**2. Requests need to be initiated by the Participants**

With gRPC since the *Coordinator* implements the gRPC server all calls need to
be initiated by the client. So we will need for the *Participant* to implement
some form of polling mechanisms to know when the *Coordinator* is ready to
start a round. Again the same solutions as the previous point can be applied.

One solution would be to block during a method call until the *Coordinator*
initiates a round.

The other solution that we eventually chose was to reuse the heartbeat
mechanism to notify the *Participants* on when to start training. During the
heartbeat messages the *Coordinator* advertises it's state with the
*Participants*. When the *Participants* see that a new round has started they
can request the global model and start their training task.


Coordinator Logic Implementation
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^

Internally the *Coordinator* is implement as a state machine that reacts to
messages sent by *Participants*.

Supplementary thoughts about interaction between coordinator :math:`C` and a
participant :math:`P`, given the gRPC-based setup described in above. First
let's consider the basic lifecycle of state transitions in :math:`C`. Let
:math:`N` be the number of required participants.

.. mermaid::

    graph TB
    A( ) -->|startup| B(STANDY)
    B -->|N Ps registered| C(ROUND N)
    C -->|all Ps trained| D(ROUND N + 1)
    D -->|no more rounds| E(FINISHED)

Once :math:`C` starts up, it's in the **STANDBY** state, waiting for incoming
connections from participants looking to rendezvous. Once :math:`N` have been
registered, a number of these are selected for a training round. To simplify
for now, assume all :math:`N` will participate.

In the **ROUND N** state, :math:`C` starts to accept requests (from the
registered :math:`N`) to start training. Any further requests (from late
entrants) to rendezvous are told to "try again later". For any :math:`P` that
has started training, :math:`C` will also accept a subsequent request of it
having finished training.

Once all :math:`N` have finished training, :math:`C` collects together all the
trained data and aggregates them generating a new global model.  It either
increments the round and repeats, or if there are more rounds to go, it
transitions to the **FINISHED** state signaling the participants to disconnect.

So far we've only discussed the lifecycle of a *successful* interaction with
all participants i.e. without faults, dropouts, etc. The true picture (taking
into account of `fault tolerance <https://hackmd.io/gzGSJZ2xQTyERNjTpqguqg>`_)
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

---

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
this state as long as it keeps receiving :code:`STANDBY` heartbeats. At some round
:math:`i`, :math:`C` may select :math:`P` for the round by responding with a :code:`ROUND` :math:`i`
heartbeat. At this point, :math:`P` moves to **Training** where the above sequence of
training messages (:code:`StartTraining` :math:`\rightarrow \theta \rightarrow \theta'
\rightarrow` :code:`EndTraining`) occur. Having received the :code:`EndTraining` reply from
:math:`C`, :math:`P` makes an "internal" transition to **Post-training** where it waits
until the start of the next round. If it has been selected again, it will
observe :code:`ROUND` :math:`i+1`. If not, it observes :code:`STANDBY`. Alternatively, if round
:math:`i` was the last, it instead sees :code:`FINISHED` and :math:`P` is **Done**. Note that
:code:`FINISHED` can also be observed from **Wait for Selection** but the transition
from there to **Done** is omitted in the diagram just for sake of clarity.
