import time
from typing import Optional

from numproto import ndarray_to_proto, proto_to_ndarray

from xain.network import client, stream_pb2


def train() -> Optional[int]:
    reconnect_in = None

    with client.connection() as c:
        consume, dispatch = c

        # Passing initiative to server with an empty init message
        dispatch(init_message())

        while True:
            # Get instruction from server
            instruction = consume()

            # Do something with instruction
            if instruction.HasField("train_config"):
                dispatch(ml_training(instruction))
            elif instruction.HasField("reconnect_in"):
                reconnect_in = instruction.reconnect_in
                break
            else:
                dispatch(unkown_instuction_message())
                break

    return reconnect_in


def init_message():
    return stream_pb2.ParticipantMessage()


def ml_training(instruction):
    theta = [proto_to_ndarray(nda) for nda in instruction.train_config.theta]

    new_thetas = []

    for t in theta:
        t = t + 1
        new_thetas.append(t)

    epoch = 10

    for i in range(epoch):
        print(f"Training... {i+1}/{epoch}")
        time.sleep(0.01)

    train_result = stream_pb2.ParticipantMessage.TrainResult(
        theta=[ndarray_to_proto(nda) for nda in new_thetas]
    )

    return stream_pb2.ParticipantMessage(result=train_result)


def unkown_instuction_message():
    return stream_pb2.ParticipantMessage(unknown_instruction=True)


def main():
    while True:
        reconnect_in = train()

        if reconnect_in is None:
            break

        print(f"Reconnecting in {reconnect_in}")
        time.sleep(reconnect_in)
