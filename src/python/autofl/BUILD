import platform


use_gpu = platform.system() == "Linux"
third_party_tf = "3rdparty/python:tensorflow"
if use_gpu:
  third_party_tf += "-gpu"


python_binary(
  name = "main",
  dependencies = [
    third_party_tf,
  ],
  source = "main.py",
)
