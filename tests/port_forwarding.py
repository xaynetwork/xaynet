"""
This module provides logic to perform port forwarding. We use this to
artificially open new TCP connections when creating gRPC clients.
"""
from errno import EBADF, ECONNRESET, ENOTCONN
import faulthandler
import socket
from threading import Event, Thread

# Buffer size for reading from a connection
BUFFER_SIZE = 4096


def transfer_worker(src, dst, terminate_event):
    """A worker that reads data from the `src` socket and forwards it to
    the `dst` socket.

    Args:

        src (socket.socket): source socket
        dst (socket.socket): destination socket
        terminate_event (Event): event used to tell the caller that
            the `src` socket is closed

    """
    while True:
        try:
            # Block until there's data to read OR the connection
            # closes.
            data = src.recv(BUFFER_SIZE)
        except OSError as exc:
            if exc.errno in [ECONNRESET, EBADF, ENOTCONN]:
                terminate_event.set()
                return
            raise

        # The socket is closed
        if not data:
            terminate_event.set()
            return

        try:
            dst.send(data)
        except OSError as exc:
            if exc.errno in [ECONNRESET, EBADF, ENOTCONN]:
                terminate_event.set()
                return
            raise


def forward(host, port, target_host, target_port, terminate_event):
    """Set up a TCP socket listening on host:port. Once a connection is
    established, open a connection to target_host:target_port and
    forward data both way.

    # Example

    ```python
    >>> forward("localhost", 8080, "localhost", 80)
    ```

    has the same effect than:

    ```shell
    socat tcp-listen:8080,reuseaddr,fork tcp:localhost:80
    ```

    Args:

        host (str): hostname or ip address for listening
        port (str): port number for listening
        target_host (str): hostname of ip address to establish a
            connection with
        target_port (str): port number to establish a connection to
            connection with
        terminate_event (Event): when this event is set, this function:
            - closes the sockets it opened
            - waits for the two `transfer_worker()` threads to finish
            - returns
    """
    # Set up a server socket that will wait for incoming connections
    server = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    server.setsockopt(socket.SOL_SOCKET, socket.SO_REUSEADDR, 1)
    # Make the socket non-blocking so that `accept()` doesn't block
    # forever, and we can periodically check whether the
    # `terminate_event` is set.
    server.settimeout(0.1)
    server.bind((host, port))
    # Allow only one connection
    server.listen(1)

    # Block until the first client connects or we're told to terminate
    while True:
        try:
            client_conn, _ = server.accept()
        except socket.timeout:
            # Check whether we should terminate
            if terminate_event.is_set():
                server.close()
                return
        else:
            break

    # Once the client is connected, open a connection with the target
    target_conn = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    target_conn.connect((target_host, target_port))

    # Start transfering data both way
    tx_thread = Thread(
        target=transfer_worker,
        name=f"{target_host}:{target_port}->{host}:{port}",
        args=(target_conn, client_conn, terminate_event),
        daemon=True,
    )
    rx_thread = Thread(
        target=transfer_worker,
        name=f"{host}:{port}->{target_host}:{target_port}",
        args=(client_conn, target_conn, terminate_event),
        daemon=True,
    )
    tx_thread.start()
    rx_thread.start()

    # Wait until we're told to terminate (this event can be fired for
    # the `transfer_worker()` threads, or by the caller.
    _ = terminate_event.wait()

    # Close all the sockets
    for sock in [target_conn, client_conn, server]:
        try:
            sock.shutdown(socket.SHUT_RDWR)
        except OSError as exc:
            if exc.errno not in [ECONNRESET, EBADF, ENOTCONN]:
                raise
        sock.close()

    # Wait for the transfer workers to terminate
    tx_thread.join(timeout=0.2)
    assert not tx_thread.isAlive()
    rx_thread.join(timeout=0.2)
    assert not rx_thread.isAlive()


class ConnectionManager:
    """
    Manage multiple forwarding workers.
    """

    def __init__(self):
        self.forwarders = {}

    def start(self, host, port, target_host, target_port):
        """Start a new forwarding worker.

        Args:

            host (str): hostname or ip address for listening
            port (str): port number for listening
            target_host (str): hostname of ip address to establish a
                connection with
            target_port (str): port number to establish a connection
                to connection with

        """
        terminate_event = Event()
        thread = Thread(
            target=forward,
            name=f"{host}:{port}<->{target_host}:{target_port}",
            args=(host, port, target_host, target_port, terminate_event),
            daemon=True,
        )
        thread.start()
        self.forwarders[(host, port)] = (terminate_event, thread)

    def stop(self, host, port):
        """Stop the forwarding worker listening on the given host and port

        Args:
            host (str): hostname or ip address the worker is listening
                on
            port (str): port number the worker is listening on

        """
        terminate_event, thread = self.forwarders.pop((host, port))
        terminate_event.set()
        thread.join(timeout=5)

    def stop_all(self):
        """
        Stop all the forwarding workers
        """
        threads = []
        for (host, port) in self.forwarders:
            terminate_event, thread = self.forwarders[(host, port)]
            terminate_event.set()
            threads.append(thread)
        for thread in threads:
            thread.join(timeout=1)
            # FIXME: sometimes some threads don't terminate in CI, so
            # we're dumping the stacktraces for all the current
            # threads for debugging purpose.
            if thread.isAlive():
                faulthandler.dump_traceback()
                raise Exception(f"Thread {thread.name} is still alive")
        self.forwarders = {}
