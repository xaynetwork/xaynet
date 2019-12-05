import numpy as np

from xain_fl.fl.coordinator.controller import RandomController


def test_random_controller():
    """Tests that the length of selected ids equals num_ids_to_select
    and that there's no replacement.
    """
    participant_ids = ["a", "b", "c", "d", "e", "f", "g"]
    for fraction in np.arange(0.25, 1, 0.25):
        controller = RandomController(participant_ids, fraction_of_participants=fraction)
        ids = controller.select_ids()

        assert len(ids) == controller.num_ids_to_select
        assert len(set(ids)) == len(ids)
