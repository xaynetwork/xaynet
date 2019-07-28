import tensorflow as tf

from . import prep


def test_init_dataset(mock_dataset):
    # Prepare
    (x, y), _ = mock_dataset

    print(x.shape, y.shape)

    # Execute
    ds = prep.init_ds_train(x, y)
    # Assert
    assert ds.output_types == (tf.float32, tf.float32)
    assert len(ds.output_shapes) == 2
    shape_x = ds.output_shapes[0]
    assert len(shape_x) == 4
    shape_y = ds.output_shapes[1]
    assert len(shape_y) == 2
