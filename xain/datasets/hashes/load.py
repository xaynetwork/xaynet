import glob
import json
import os


def load_hashes():
    hashes = {}
    path = os.path.dirname(__file__)

    json_pattern = os.path.join(path, "*.json")
    file_list = glob.glob(json_pattern)

    for fname in file_list:
        with open(fname) as f:
            dict_key = os.path.basename(fname)[0:-5]
            hashes[dict_key] = json.loads(f.read())

    return hashes
