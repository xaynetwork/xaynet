from .coordinator import history_update


def test_():
    # Prepare
    h = {
        "acc": [0.31569222, 0.3951613, 0.4074261],
        "loss": [2.6372881730397544, 1.8302593352974101, 1.5969073498441326],
        "val_acc": [0.22115384, 0.27584136, 0.36418268],
        "val_loss": [2.0736552010744047, 2.0123409873399978, 1.647030892280432],
    }
    u = {
        "acc": [0.31569222, 0.3951613, 0.4074261],
        "loss": [2.6372881730397544, 1.8302593352974101, 1.5969073498441326],
        "val_acc": [0.22115384, 0.27584136, 0.36418268],
        "val_loss": [2.0736552010744047, 2.0123409873399978, 1.647030892280432],
    }
    expected = {
        "acc": [0.31569222, 0.3951613, 0.4074261] * 2,
        "loss": [2.6372881730397544, 1.8302593352974101, 1.5969073498441326] * 2,
        "val_acc": [0.22115384, 0.27584136, 0.36418268] * 2,
        "val_loss": [2.0736552010744047, 2.0123409873399978, 1.647030892280432] * 2,
    }
    # Execute
    actual = history_update(h, u)
    # Assert
    assert expected == actual
