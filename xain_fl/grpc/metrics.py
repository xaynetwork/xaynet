import time
import random
from datetime import datetime
from influxdb import InfluxDBClient

INFLUXDB_HOST = "10.10.100.164"
INFLUXDB_PORT = 8086
INFLUXDB_DATABASE = "xain"


def current_time():
    return datetime.utcnow().strftime('%Y-%m-%dT%H:%M:%SZ')


def write_number_of_participants(client, number_of_participants):
    message = {
        "measurement": "coordinator",
        "tags": {
            "host": "10.10.100.116:50051"
        },
        "time": current_time(),
        "fields": {
            "number_of_participants": number_of_participants
        }
    }
    return client.write_points([message], database=INFLUXDB_DATABASE)


def write_round_number(client, round_number, total_rounds):
    message = {
        "measurement": "coordinator",
        "tags": {
            "host": "10.10.100.116:50051"
        },
        "time": current_time(),
        "fields": {
            "round_number": round_number,
            "total_rounds": total_rounds,
        }
    }
    return client.write_points([message], database=INFLUXDB_DATABASE)


def write_coordinator_state(client, state):
    message = {
        "measurement": "coordinator",
        "tags": {
            "host": "10.10.100.116:50051"
        },
        "time": current_time(),
        "fields": {
            "state": state
        }
    }
    return client.write_points([message], database=INFLUXDB_DATABASE)


def write_participant_state(client, state, participant_id):
    message = {
        "measurement": "participant",
        "tags": {
            "host": participant_id
        },
        "time": current_time(),
        "fields": {
            "state": state
        }
    }
    return client.write_points([message], database=INFLUXDB_DATABASE)


def write_participant_round(client, round_number, participant_id):
    message = {
        "measurement": "participant",
        "tags": {
            "host": participant_id
        },
        "time": current_time(),
        "fields": {
            "round_number": round_number
        }
    }
    return client.write_points([message], database=INFLUXDB_DATABASE)


def write_participant_loss(client, loss, participant_id):
    message = {
        "measurement": "participant",
        "tags": {
            "host": participant_id
        },
        "time": current_time(),
        "fields": {
            "loss": loss
        }
    }
    return client.write_points([message], database=INFLUXDB_DATABASE)


def write_participant_accuracy(client, accuracy, participant_id):
    message = {
        "measurement": "participant",
        "tags": {
            "host": participant_id
        },
        "time": current_time(),
        "fields": {
            "accuracy": accuracy
        }
    }
    return client.write_points([message], database=INFLUXDB_DATABASE)



def test_metrics():
    client = InfluxDBClient(INFLUXDB_HOST, INFLUXDB_PORT)
    client.create_database(INFLUXDB_DATABASE)

    number_of_participants = 0
    while True:
        number_of_participants += random.choice([-1, 1]) * random.randint(1, 3)
        number_of_participants = max(0, number_of_participants)
        write_number_of_participants(client, number_of_participants)
        print(f"writing {number_of_participants}")
        time.sleep(2)


if __name__ == "__main__":
    test_metrics()
