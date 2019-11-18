import hashlib


def checksum(fpath: str) -> str:
    """Return checksum of file a fpath

    Args:
        fpath (str): absolute path to file

    Returns:
        str: sha1.hexdigest() of file
    """
    sha1 = hashlib.sha1()

    with open(fpath, "rb") as f:
        while True:
            data = f.read()
            if not data:
                break
            sha1.update(data)

    return sha1.hexdigest()
