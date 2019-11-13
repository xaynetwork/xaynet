# Network Module

This module encapsulates the networking logic required for the training on multiple machines. It provides a context manager for the client and a client manager, client manager factory as well as a ParticipantProxy which holds participant/server messages until they are processed.

## client.connection (Context Manager)

Provides a connection context while returning a tuple of two with two methods named `consume()` and `dispatch(proto_message)`.

- **consume():** Returns a server side instruction
- **dispatch(proto_message):** Takes a proto message which is send as a response to the server

```python
    with client.connection() as c:
        consume, dispatch = c

        # Passing initiative to server with an empty init message
        dispatch(stream_pb2.ParticipantMessage())

        while True:
            # Get next instruction
            instruction = consume()

            # Do something now with instruction...
            results = do_something(instruction)

            # Dispatch the results
            dispatch(results)
```

## ParticipantProxy

ParticipantProxy is a abstract base class which exposes a method called `run(proto_message)` and is the recommended way of implementing the server side client proxy.

Usage example:

```python
class Participant(ParticipantProxy):
    """Holds request until its anwered"""
    def train(self, theta):
        instruction = stream_pb2.ServerMessage()
        response_from_client_as_pb_message = self.run(instruction)
        return response_from_client_as_pb_message
```

## ParticipantManager

ParticipantManager is a subclass of the gRPC servicer which will be added to the gRPC server when calling the `server.create_participant_manager()` function and returned. It exposes only one relevant method `participant_manager.get_participants(min_num_participants: int)` which will block until it can return the minimum number of participants requested. It can be used as in the following example

```python
    def participant_factory():
        participant_instance = Participant()
        return participant_instance

    _server, participant_manager = create_participant_manager(
        participant_proxy_factory=participant_factory
    )
```

## Communication over gRPC stream (request/response model)

Technically each instruction send by the server is a response to the previous participant request. Partically we are holding each participant request on the serverside until we have a new instruction to be send.
We initialize the flow with an empty participant request send to the server.
![Sequence Diagram](sequence_diagram.jpg)

## FAQ

#### How many CPU cores does the participant_manager utilize?

Due to the GIL in Python the gRPC server is limited to a single CPU core even though we are using the ThreadPoolExecutor. To enable usage of all cores the architecture will have to be extended to a global coordinator with multiple participant managers.
