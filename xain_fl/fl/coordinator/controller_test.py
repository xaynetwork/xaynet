import numpy as np
import pytest

from xain_fl.fl.coordinator.controller import RandomController


def test_random_controller():
    """Tests that the length of selected ids is correct and that
    there's no replacement.
    """
    participant_ids = ["a", "b", "c", "d", "e", "f", "g"]
    fractions = np.arange(0.25, 1, 0.25)
    expected_lengths = [
        np.ceil(fraction * len(participant_ids)) for fraction in fractions
    ]
    for fraction, expected_length in zip(fractions, expected_lengths):
        controller = RandomController(
            participant_ids, fraction_of_participants=fraction
        )
        ids = controller.select_ids()
        set_ids = set(ids)

        # check that length of set_ids is as expected
        assert len(set_ids) == expected_length

        # check that every element of set_ids belongs to participant_ids
        assert set_ids.issubset(participant_ids)


def test_select_from_empty_list():
    """Tests that if participant_ids is a list we are unable to select a subset of it
    (due to numpy's ValueError)
    """
    participant_ids = []
    controller = RandomController(participant_ids, fraction_of_participants=1.0)

    # we expect numpy.random.choice() used in select_ids() to raise a ValueError
    with pytest.raises(ValueError):
        controller.select_ids()
