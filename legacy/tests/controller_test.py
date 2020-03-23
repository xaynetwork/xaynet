"""XAIN FL tests for controller"""

import numpy as np

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
        controller = RandomController(fraction_of_participants=fraction)
        ids = controller.select_ids(participant_ids)
        set_ids = set(ids)

        # check that length of set_ids is as expected
        assert len(set_ids) == expected_length

        # check that every element of set_ids belongs to participant_ids
        assert set_ids.issubset(participant_ids)
