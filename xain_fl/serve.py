"""XAIN FL serving"""

from concurrent import futures
import threading
import time

import grpc
from xain_proto.fl import coordinator_pb2_grpc

from xain_fl.coordinator import _ONE_DAY_IN_SECONDS
from xain_fl.coordinator.coordinator import Coordinator
from xain_fl.coordinator.coordinator_grpc import CoordinatorGrpc
from xain_fl.coordinator.heartbeat import monitor_heartbeats
from xain_fl.coordinator.store import Store
from xain_fl.logger import StructLogger, get_logger

logger: StructLogger = get_logger(__name__)


def serve(coordinator: Coordinator, store: Store, host: str = "[::]", port: int = 50051) -> None:
    """Main method to start the gRPC service.

    This methods just creates the :class:`xain_fl.coordinator.coordinator.Coordinator`,
    sets up all threading events and threads and configures and starts the gRPC service.
    """
    terminate_event = threading.Event()
    monitor_thread = threading.Thread(
        target=monitor_heartbeats, args=(coordinator, terminate_event)
    )

    server = grpc.server(futures.ThreadPoolExecutor(max_workers=10))
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(
        CoordinatorGrpc(coordinator, store), server
    )
    server.add_insecure_port(f"{host}:{port}")
    server.start()
    monitor_thread.start()

    logger.info("Coordinator waiting for connections...")

    try:
        while True:
            time.sleep(_ONE_DAY_IN_SECONDS)
    except KeyboardInterrupt:
        terminate_event.set()
        server.stop(0)
