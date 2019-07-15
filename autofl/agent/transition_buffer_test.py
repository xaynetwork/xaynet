from .transition_buffer import TransitionBuffer


def test_transition_buffer_len_empty():
    # Prepare
    capacity = 10
    buffer = TransitionBuffer(capacity)
    # Execute &
    length = len(buffer)
    # Assert
    assert length == 0


def test_transition_buffer_len_one():
    # Prepare
    capacity = 10
    buffer = TransitionBuffer(capacity)
    t = (0, 0, 0.0, 1, False)
    # Execute
    buffer.store(t)
    # Assert
    assert len(buffer) == 1
