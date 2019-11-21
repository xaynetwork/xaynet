# Hackaton

## Code and Installation

The code from the XAIN side is under the
[hackaton-topic1](https://github.com/xainag/xain/tree/hackaton-topic1<Paste>)
and the hackaton related code will be under the [hackaton
folder](https://github.com/xainag/xain/tree/hackaton-topic1/hackaton).

**Install with pip:**

```bash
$ pip install git+https://github.com/xainag/xain.git@hackaton-topic1
```

**Using the participant**

To use the grpc participant:
```python
from xain.fl.participant import Participant
from xain.grpc.participant import go

# Create a participant
participant = Participant(...)

# To connect to the coordinator call the `go` method passing as arguments the
# participant and the address of the coordinator in the form of "ip:port"
go(participant, "10.10.10.10:50051")
```
