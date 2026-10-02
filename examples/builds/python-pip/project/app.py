from packaging.version import Version


def supported_version(value):
    return Version(value) >= Version("1.0")


def health():
    return {"status": "ok"}


if __name__ == "__main__":
    print(health())
