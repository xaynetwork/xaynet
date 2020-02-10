"""XAIN FL serving"""

from concurrent import futures
import threading

import grpc
from structlog import get_logger
from xain_proto.fl import coordinator_pb2_grpc

from xain_fl.config import ServerConfig
from xain_fl.coordinator.coordinator import Coordinator
from xain_fl.coordinator.coordinator_grpc import CoordinatorGrpc
from xain_fl.coordinator.heartbeat import monitor_heartbeats
from xain_fl.logger import StructLogger

logger: StructLogger = get_logger(__name__)


def serve(coordinator: Coordinator, server_config: ServerConfig) -> None:
    """Start a coordinator service and keep it running until an
    interruption signal (``SIGINT``) is received.

    Args:

        coordinator:
            :class:`xain_fl.coordinator.coordinator.Coordinator`
            instance to run

        server_config:
            server configuration: binding address, gRPC options, etc.

    """
    server = grpc.server(
        futures.ThreadPoolExecutor(max_workers=10), options=server_config.grpc_options
    )
    coordinator_pb2_grpc.add_CoordinatorServicer_to_server(
        CoordinatorGrpc(coordinator), server
    )
    server.add_insecure_port(f"{server_config.host}:{server_config.port}")
    server.start()

    logger.info("Coordinator waiting for connections...")

    terminate_event = threading.Event()
    try:
        monitor_heartbeats(coordinator, terminate_event)
    except KeyboardInterrupt:
        terminate_event.set()
        server.stop(0)
