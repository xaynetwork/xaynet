REST API
===============

This document explains which Coordinator and Aggregator API endpoints are available and how to use
them.

Swagger UI
----------

The docker-compose setup includes a Swagger UI container that can be used to view the Swagger files.
After you have started docker-compose via the command
``docker-compose -f docker/docker-compose.yml up`` in the root of the repository, you can view the
Swagger UI in your browser at the following address: http://127.0.0.1/.

Coordinator API Reference
-------------------------

.. openapi:: coordinator.yml


Aggregator API Reference
------------------------

.. openapi:: aggregator.yml
