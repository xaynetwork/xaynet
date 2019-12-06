import numpy as np

from xain_fl.fl.coordinator.controller import RandomController


def test_random_controller():
    """Tests that the length of selected ids is correct and that
    there's no replacement.
    """
    participant_ids = ["a", "b", "c", "d", "e", "f", "g"]
    fractions = np.arange(0.25, 1, 0.25)
    len_ids_selected = [np.ceil(fraction * len(participant_ids)) for fraction in fractions]
    for fraction, len_ids in zip(fractions, len_ids_selected):
        controller = RandomController(participant_ids, fraction_of_participants=fraction)
        ids = controller.select_ids()

        assert len(ids) == len_ids
        assert len(set(ids)) == len(ids)
