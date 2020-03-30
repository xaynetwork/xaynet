Tutorial
=========

In this tutorial, we'll write a participant that can be used with the
XAIN FL Platform.

Setup
-----

To follow this tutorial we need:

- docker-compose
- python (3.6 or higher)

To ease the setup, we'll use the ``xain-sdk-tutorial`` repository:

.. code-block:: bash

          git clone https://github.com/xainag/xain-sdk-tutorial/
          cd xain-sdk-tutorial

Installing ``xain-sdk``
^^^^^^^^^^^^^^^^^^^^^^^

For this tutorial we recommend using a virtual
environment. ``xain-sdk`` can be directly installed from pypi:

.. code-block:: bash

          pip install xain-sdk==0.8.0


.. _running-xain-fl:

Running the XAIN FL Platform
^^^^^^^^^^^^^^^^^^^^^^^^^^^^

To test our participant, we'll need the XAIN FL Platform to be
running. The repository contains a docker-compose file for this:

.. code-block:: bash

          docker-compose up

The output should look like:

.. code-block:: none

          Starting xain-tutorial_coordinator_1 ... done
          Starting xain-tutorial_aggregator_1  ... done
          Attaching to xain-tutorial_coordinator_1, xain-tutorial_aggregator_1
          coordinator_1  | 2020-03-30T13:18:54.830743280+00:00  ERROR stubborn_io::tokio::io: Initial connection failed due to: Os { code: 111, kind: ConnectionRefused, message: "Connection refused" }.
          coordinator_1  | 2020-03-30T13:18:54.830901973+00:00   INFO stubborn_io::tokio::io: Will re-perform initial connect attempt #1 in 1s.
          aggregator_1   | 2020-03-30T13:18:54.856655739+00:00   INFO stubborn_io::tokio::io: Initial connection succeeded.
          aggregator_1   | 2020-03-30T13:18:54.857290471+00:00   INFO xain_fl::aggregator::api: starting HTTP server on 0.0.0.0:8082
          aggregator_1   | 2020-03-30T13:18:54.857353457+00:00   INFO warp::server: listening with custom incoming
          aggregator_1   | INFO:PythonWeightedAverageAggregator:initializing aggregator
          coordinator_1  | 2020-03-30T13:18:55.833312001+00:00   INFO stubborn_io::tokio::io: Attempting reconnect #1 now.
          coordinator_1  | 2020-03-30T13:18:55.835300030+00:00   INFO stubborn_io::tokio::io: Initial connection successfully established.
          coordinator_1  | 2020-03-30T13:18:55.836805361+00:00   INFO xain_fl::coordinator::api: starting HTTP server on 0.0.0.0:8081
          coordinator_1  | 2020-03-30T13:18:55.837086682+00:00   INFO warp::server: listening with custom incoming

That's it, the platform is running! But before diving in, let's
introduce the key concepts that power Federated Learning.

XAIN Federated Learning in a nutshell
-------------------------------------

.. note::

   This section is a very quick introduction to the XAIN FL
   Platform. A more in-depth description is available
   `on XAIN's website <https://www.xain.io/federated-learning-technology>`_.

Federated Learning is a distributed machine learning approach. In its
simplest form it is composed of a *coordinator* and a set of
*participants*. The coordinator is responsible for keeping any state
required for the machine learning task, orchestrate the machine
learning task across a set of participants, and perform the
aggregation of the individual updates returned by the participants.

In the XAIN FL Platform, the coordinator performs several rounds of
training. For each round, it selects a subset of all the
participants. These participants retrieve the latest global ML model
from the coordinator, train on their local data, update the ML model
locally, and finally send it back to the coordinator. Once all the
participants selected by the coordinator have sent their results, the
coordinator aggregates them to produce a new global ML model.


.. _lifecycle:

Participant lifecyle
^^^^^^^^^^^^^^^^^^^^^

In this tutorial, we're interested in writing *participants*. So let's
take a closer look at a participant's lifecycle. When it starts, a
participant should follow these steps:


1. Connect to the coordinator
2. Wait for being selected by the coordinator to take part in a round
   of training
3. Once selected, retrieve the latest training data from the
   coordinator, in particular the model weights
4. Train
5. Send the training results to the coordinator, then go back to step
   ``2.``

With this knowledge, we're ready to start coding.

Goal
----

To keep things simple, the participant we're going to implement won't
solve a real machine learning problem. The idea is to write a
minimalistic working example, that demonstrates that the system works.

The model we'll use is a simple array with 5 identical ``float``
values (for instance ``[1.2, 1.2, 1.2, 1.2, 1.2]``). At the beginning of round ``i``,
let's suppose that the global model is ``[100.0, 100.0, 100.0, 100.0, 100.0]``. The participants that are selected to take part in round ``i`` will retrieve this model, pick a value between ``0`` and ``100.0``, and return an array with that value. For instance if a participant picks ``15.5``, it would send back to the coordinator an array filled with that value: ``[15.5, 15.5, 15.5, 15.5, 15.5]``.

Since the coordinator aggregates the participant models by computing
their average at each round, the global model should gradually
converge toward ``[0, 0, 0, 0, 0]`` if the system works correctly.


Implementation
--------------

Getting started
^^^^^^^^^^^^^^^

Let's get to work. The repository we cloned earlier already contains
the skeleton of a python package to get us started:

.. code-block:: none

        .
        ├── setup.py
        └── xain-sdk-tutorial
           ├── __init__.py
           └── participant.py


We'll first install that package in development mode (with the ``-e`` flag):


.. code-block:: none

                pip install -e .


This should install the dependencies we'll need and make a
``run-participant`` command available:


.. code-block:: none

  $ run-participant --help
  usage: run-participant [-h] [--upper-bound UPPER_BOUND]
  run a participant
  optional arguments:
    -h, --help            show this help message and exit
    --upper-bound UPPER_BOUND
                          Initial upper bound for picking a random float to send
                          to the coordinator

This is the command we'll use to test our participants.

The ``participant.py`` module currently contains some boilerplate
code:

.. code-block:: Python

  import argparse
  import logging
  import os

  LOG = logging.getLogger(__name__)


  class Participant:
      def __init__(self, initial_upper_bound: float) -> None:
          super(Participant, self).__init__()
          self.upper_bound = initial_upper_bound


  def main():
      logging.basicConfig(
          format="%(asctime)s.%(msecs)03d %(levelname)-8s %(message)s",
          level=logging.DEBUG,
          datefmt="%Y-%m-%d %H:%M:%S",
      )

      parser = argparse.ArgumentParser(description="run a participant")

      parser.add_argument(
          "--upper-bound",
          required=True,
          type=float,
          help="Initial upper bound for picking a random float to send to the coordinator",
      )
      args = parser.parse_args()


  if __name__ == "__main__":
      main()

``main()`` is the function that is called by the ``run-participant``
command and we already have a ``Participant`` class defined.


The ``ParticipantABC`` class
^^^^^^^^^^^^^^^^^^^^^^^^^^^^

As explained earlier in the
:ref:`participant lifecycle paragraph <lifecycle>`,
a participant needs to communicate with the coordinator. ``xain-sdk``
already implements this logic so all we need to do is implement the
``xain-sdk.ParticipantABC`` abstract class, which looks like this:

.. code-block:: Python

    from abc import ABC, abstractmethod
    from typing import TypeVar

    TrainingResult = TypeVar("TrainingResult")
    TrainingInput = TypeVar("TrainingInput")

    class ParticipantABC(ABC):
        @abstractmethod
        def train_round(self, training_input: TrainingInput) -> TrainingResult:
            raise NotImplementedError()

        @abstractmethod
        def serialize_training_result(self, training_result: TrainingResult) -> bytes:
            raise NotImplementedError()

        @abstractmethod
        def deserialize_training_input(self, data: bytes) -> TrainingInput:
            raise NotImplementedError()

There are three methods to implement.

The most important one is ``train_round``, which takes any type of
object (named ``TrainingInput`` for clarity), and returns a result,
which can also be any type of object (named ``TrainingResult`` for
clarity as well). **This is the method that the SDK will call to perform
the training**. The ``training_input`` argument will be the global model
retrieved from the coordinator. The training result returned by
``train_round`` will be sent to the coordinator.

Then come the methods used for data (de)serialization:

- ``deserialize_training_input`` is called right after the SDK has
  downloaded the latest global model from the coordinator. It is used
  to deserialize the data that will be passed to ``train_round``.
- ``serialize_training_result`` is its counterpart: it is called by
  the SDK to serialize the value returned by ``train_round``, so that
  it can be sent to the coordinator.

.. note::

   The reason these two methods exist is because there is no
   limitation on the formats that can be used for the communications
   between the coordinator and the participants. This is how the XAIN
   FL Platform can handle such a wide variety of Federated Learning
   use cases: users have full control on what data is exchanged, and
   how the coordinator performs the aggregation of all this data,
   although this is out of scope of this document.

Before implementing these methods, let's make our ``Participant``
inherit from ``xain-sdk.ParticipantABC``:

.. code-block:: Python

  # xain_sdk_tutorial/participant.py

  import argparse
  import logging
  import os
  import xain_sdk


  LOG = logging.getLogger(__name__)


  # Inherit from ParticipantABC
  class Participant(xain_sdk.ParticipantABC):
      def __init__(self, initial_upper_bound: float) -> None:
          super(Participant, self).__init__()
          self.upper_bound = initial_upper_bound



Data serialization
^^^^^^^^^^^^^^^^^^

To implement the (de)serialization methods we need to know what
messages are being exchanged between the coordinator and the
participants. The coordinator we started in the :ref:`Running the
XAIN-FL Platform <running-xain-fl>`:

- expects the participants to send the concatenation of:
  - an ``int`` that represents the number of samples on which the participant trained, serialized with :py:meth:`int.to_bytes`
  - the weights of the model that the participant trained, as a flat numpy array serialized with :py:func:`numpy.save`,
- sends to the participants the global model as a numpy array serialized with :py:func:`numpy.save`

Therefore, our serialization methods will look like:

.. code-block:: Python

    # xain_sdk_tutorial/participant.py

    # A buffer used for the (de)serialization process
    from io import BytesIO

    # In this tutorial we use type annotations to help better
    # understand the data flow, but it is optional
    from typing import Tuple

    import numpy as np

    def deserialize_training_input(self, data: bytes) -> np.ndarray:
        # numpy.load takes a file-like object as argument, so we
        # wrap the data to deserialize into a BytesIO
        reader = BytesIO(data)
        return np.load(reader, allow_pickle=False)

    def serialize_training_result(self, training_result: Tuple[int, np.ndarray]) -> bytes:
        # Our `train_round` method will return a tuple where:
        #   - the first item will be an integer that represents the number of samples on which the participant trained
        #   - the second item represents the model trained by the participant
        (number_of_samples, weights) = training_result

        # The writer holds the buffer into which we'll write the serialized data
        writer = BytesIO()

        # The coordinator expects the number of samples to be encoded on 4 bytes, in BigEndian
        writer.write(number_of_samples.to_bytes(4, byteorder="big"))

        # Append the numpy array
        np.save(writer, weights, allow_pickle=False)

        # Return the entire buffer
        return writer.getbuffer()[:]



Training
^^^^^^^^

We can now focus on the most interesting method: the one where
training happens. In our case, we'll just generate partially random
data as explained in `the "Goals" section <goals>`_.

We want to generate an array of 5 identical float numbers, between 0
and some upper bound from the latest global model:

.. code-block:: Python

   # xain_sdk_tutorial/participant.py

   import random

   # ...

   def train_round(self, global_weights: np.ndarray) -> Tuple[int, np.ndarray]:
       # Get the upper bound from the global model:
       self.upper_bound = global_weights[0]

       # Pick a random value
       value = random.uniform(0.0, self.upper_bound)

       # Create the model to send to the coordinator
       local_weights = np.repeat(value, 5)

       # The coordinator also expects the number of samples the
       # participant trained on, but we're not actually doing any
       # training, so let's hardcode this to 1
       number_of_samples = 1

       return (number_of_samples, local_weights)



Starting the participant
^^^^^^^^^^^^^^^^^^^^^^^^

Currently, our ``main()`` function doesn't do anything apart from
parsing the CLI arguments. Let's instantiate our participant, and
start it with ``xain_sdk.run_participant``. We also set up some logger
with ``xain_sdk.configure_logging``:

.. code-block:: Python

    # xain_sdk_tutorial/participant.py

    def main():
        logging.basicConfig(
            format="%(asctime)s.%(msecs)03d %(levelname)-8s %(message)s",
            level=logging.DEBUG,
            datefmt="%Y-%m-%d %H:%M:%S",
        )

        parser = argparse.ArgumentParser(description="run a participant")
        parser.add_argument(
            "--upper-bound",
            required=True,
            type=float,
            help="Initial upper bound for picking a random float to send to the coordinator",
        )
        args = parser.parse_args()

        # Instantiate a participant
        participant = Participant(args.upper_bound)

        # Set up some logger so that we can see the requests being made to the coordinator
        xain_sdk.configure_logging(log_http_requests=True)

        # Start the participant
        coordinator_url = "http://localhost:8081"
        xain_sdk.run_participant(participant, coordinator_url)



First run
^^^^^^^^^

``participant.py`` should now look like this:

.. code-block:: Python

  # xain_sdk_tutorial/participant.py

  import argparse
  import logging
  from io import BytesIO
  import os
  import random
  from typing import Tuple

  import numpy as np
  import xain_sdk

  LOG = logging.getLogger(__name__)


  class Participant(xain_sdk.ParticipantABC):
      def __init__(self, initial_upper_bound: float) -> None:
          super(Participant, self).__init__()
          self.upper_bound = initial_upper_bound


      def deserialize_training_input(self, data: bytes) -> np.ndarray:
          reader = BytesIO(data)
          return np.load(reader, allow_pickle=False)

      def serialize_training_result(self, training_result: Tuple[int, np.ndarray]) -> bytes:
          (number_of_samples, weights) = training_result
          writer = BytesIO()
          writer.write(number_of_samples.to_bytes(4, byteorder="big"))
          np.save(writer, weights, allow_pickle=False)
          return writer.getbuffer()[:]

      def train_round(self, global_weights: np.ndarray) -> Tuple[int, np.ndarray]:
          self.upper_bound = global_weights[0]
          value = random.uniform(0.0, self.upper_bound)
          local_weights = np.repeat(value, 5)
          number_of_samples = 1
          return (number_of_samples, local_weights)

  def main():
      logging.basicConfig(
          format="%(asctime)s.%(msecs)03d %(levelname)-8s %(message)s",
          level=logging.DEBUG,
          datefmt="%Y-%m-%d %H:%M:%S",
      )

      parser = argparse.ArgumentParser(description="run a participant")
      parser.add_argument(
          "--upper-bound",
          required=True,
          type=float,
          help="Initial upper bound for picking a random float to send to the coordinator",
      )
      args = parser.parse_args()

      participant = Participant(args.upper_bound)
      xain_sdk.configure_logging(log_http_requests=True)
      coordinator_url = "http://localhost:8081"
      xain_sdk.run_participant(participant, coordinator_url)


  if __name__ == "__main__":
      main()



In another tertminal, let's start a participant, with an initial upper
bound of `100.0` with ``run-participant --upper-bound 100``. We see a
bunch of messages being exchanged, but quickly:

.. code-block::

  2020-03-31 11:19:14.966 INFO     downloading global weights from the aggregator
  2020-03-31 11:19:14.966 INFO     >>> GET http://localhost:8082/d66e5bce-3bf6-4dce-a09a-85830afbd4d5/528aff97-cf06-4334-90dd-6016f8f36a0f
  2020-03-31 11:19:14.971 INFO     <<< GET http://localhost:8082/d66e5bce-3bf6-4dce-a09a-85830afbd4d5/528aff97-cf06-4334-90dd-6016f8f36a0f [200]
  2020-03-31 11:19:14.971 DEBUG    content-type: application/octet-stream
  2020-03-31 11:19:14.971 DEBUG    content-length: 0
  2020-03-31 11:19:14.971 DEBUG    date: Tue, 31 Mar 2020 09:19:14 GMT
  2020-03-31 11:19:14.971 INFO     retrieved training data (length: 0 bytes)

  Traceback (most recent call last):
    File "/python/sdk/xain_sdk/participant.py", line 161, in train
      training_input: Any = self.participant.deserialize_training_input(data)
    File "/xain-tutorial/xain_sdk_tutorial/participant.py", line 21, in deserialize_training_input
      return np.load(reader, allow_pickle=False)
    File "/lib/python3.7/site-packages/numpy/lib/npyio.py", line 457, in load
      raise ValueError("Cannot load file containing pickled data "
  ValueError: Cannot load file containing pickled data when allow_pickle=False

The error here is slightly misleading. The deserialization failure has
nothing to do with ``pickle``. If we look at the logs, we see that
when downloading the global model from the coordinator, the response
is empty (``content-length: 0``):

.. code-block::

  2020-03-31 11:19:14.966 INFO     downloading global weights from the aggregator
  2020-03-31 11:19:14.966 INFO     >>> GET http://localhost:8082/d66e5bce-3bf6-4dce-a09a-85830afbd4d5/528aff97-cf06-4334-90dd-6016f8f36a0f
  2020-03-31 11:19:14.971 INFO     <<< GET http://localhost:8082/d66e5bce-3bf6-4dce-a09a-85830afbd4d5/528aff97-cf06-4334-90dd-6016f8f36a0f [200]
  2020-03-31 11:19:14.971 DEBUG    content-type: application/octet-stream
  2020-03-31 11:19:14.971 DEBUG    content-length: 0

It totally makes sense: this is the first round so the coordinator
doesn't have any weight yet! We have to handle this first round
as a special case somehow.

First round handling
^^^^^^^^^^^^^^^^^^^^

During the first round, the coordinator will send an empty message, so
our ``deserialize_training_input`` method will just deserialize it as
``None``:

.. code-block:: Python

    # xain_sdk_tutorial/participant.py

    # ...
    from typing import Optional, Tuple

    # ...

    def deserialize_training_input(self, data: bytes) -> Optional[np.ndarray]:
        if not data:
            return None
        reader = BytesIO(data)
        return np.load(reader, allow_pickle=False)


Of course, ``train_round`` must be updated to handle the case where
the input is ``None``:

.. code-block:: Python

    def train_round(self, global_weights: Optional[np.ndarray]) -> Tuple[np.ndarray, int]:
        if global_weights is not None:
            self.upper_bound = global_weights[0]

        value = random.uniform(0.0, self.upper_bound)
        local_weights = np.repeat(value, 5)
        number_of_samples = 1
        return (number_of_samples, local_weights)



With these changes, the participant should run correctly. Before
trying it out, let's add some logs to see if the weights are
converging toward ``0`` as we expect:

.. code-block:: Python

    def train_round(self, global_weights: Optional[np.ndarray]) -> Tuple[np.ndarray, int]:
        if global_weights is not None:
            LOG.info("global weights: %s", global_weights)
            self.upper_bound = global_weights[0]

        value = random.uniform(0.0, self.upper_bound)
        local_weights = np.repeat(value, 5)
        LOG.info("local weights %s", local_weights)
        number_of_samples = 1
        return (number_of_samples, local_weights)


The final code:

.. code-block:: Python

  import argparse
  import logging
  from io import BytesIO
  import os
  import random
  from typing import Tuple, Optional

  import numpy as np
  import xain_sdk

  LOG = logging.getLogger(__name__)


  class Participant(xain_sdk.ParticipantABC):
      def __init__(self, initial_upper_bound: float) -> None:
          super(Participant, self).__init__()
          self.upper_bound = initial_upper_bound


      def deserialize_training_input(self, data: bytes) -> Optional[np.ndarray]:
          if not data:
              return None
          reader = BytesIO(data)
          return np.load(reader, allow_pickle=False)

      def serialize_training_result(self, training_result: Tuple[int, np.ndarray]) -> bytes:
          (number_of_samples, weights) = training_result
          writer = BytesIO()
          writer.write(number_of_samples.to_bytes(4, byteorder="big"))
          np.save(writer, weights, allow_pickle=False)
          return writer.getbuffer()[:]

      def train_round(self, global_weights: Optional[np.ndarray]) -> Tuple[int, np.ndarray]:
          if global_weights is not None:
              LOG.info("global weights: %s", global_weights)
              self.upper_bound = global_weights[0]

          value = random.uniform(0.0, self.upper_bound)
          local_weights = np.repeat(value, 5)
          LOG.info("local weights %s", local_weights)
          number_of_samples = 1
          return (number_of_samples, local_weights)

  def main():
      logging.basicConfig(
          format="%(asctime)s.%(msecs)03d %(levelname)-8s %(message)s",
          level=logging.DEBUG,
          datefmt="%Y-%m-%d %H:%M:%S",
      )

      parser = argparse.ArgumentParser(description="run a participant")
      parser.add_argument(
          "--upper-bound",
          required=True,
          type=float,
          help="Initial upper bound for picking a random float to send to the coordinator",
      )
      args = parser.parse_args()

      participant = Participant(args.upper_bound)
      xain_sdk.configure_logging(log_http_requests=True)
      coordinator_url = "http://localhost:8081"
      xain_sdk.run_participant(participant, coordinator_url)


  if __name__ == "__main__":
      main()



When running with ``run-participant --upper-bound 1000``, we should
see the global weights decreasing round after round.

Going further
-------------

In this tutorial we learned how to use ``xain-sdk`` to write a participant, but that participant doesn't do real training yet. For an actual machine learning example, the the `"house pricing problem" example <https://github.com/xainag/xain-fl/tree/master/python/client_examples/keras_house_prices>`_, which uses Keras.

For more details about the architecture of the platform itself, take a look at the `xainag/xain-fl Github repository <https://github.com/xainag/xain-fl>`_

To see how to tune the coordinator (number of rounds, fraction of participants to select, etc.), take a look at the `sample configuration files in the xain-fl repository <https://github.com/xainag/xain-fl/tree/master/configs>`_.
