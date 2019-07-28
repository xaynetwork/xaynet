from .orig_2nn import orig_2nn_compiled
from .orig_cnn import orig_cnn_compiled

print("\n2NN:")
model = orig_2nn_compiled()
model.summary()

print("\nCNN (MNIST):")
model = orig_cnn_compiled()
model.summary()

print("\nCNN (CIFAR-10):")
model = orig_cnn_compiled(input_shape=(32, 32, 3))
model.summary()
