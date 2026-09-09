try:
    import pyperclip
except ImportError:
    print("Install pyperclip with `pip install pyperclip`")
    exit(1)

try:
    import ijson
except ImportError:
    print("Install simdjson with `pip install pysimdjson`")
    exit(1)

import base64, zlib
from pathlib import Path

TEST_PATH = Path("~/Downloads/giftorio-blueprint.bp").expanduser()
JSON_PATH = Path("~/Downloads/giftorio-blueprint.json").expanduser()

backend = ijson.get_backend("yajl2_c")


def decode_blueprint(s: str) -> dict:
    if (s := "".join(s.split())).startswith("0"):  # First char is Factorio blueprint version.
        s = s[1:]
    return zlib.decompress(base64.b64decode(s))


global i
i = 0


def validate_streaming(fp):
    global i
    try:
        for _ in backend.parse(fp):
            i += 1

            if i % 1000_000 == 0:
                print(i, _)
        return True
    except Exception as e:
        raise
        return False


# print(json.dumps(decode_blueprint(pyperclip.paste()), indent=4))

# raw = decode_blueprint(TEST_PATH.read_text())
# JSON_PATH.write_text(raw.decode())
# exit()
with open(JSON_PATH, "r", encoding="utf-8") as f:
    # f.seek(0, 2)  # go to end
    # pos = f.tell()
    # f.seek(max(0, pos - 20000))
    text = f.read(5000)


print()
print()
print()
print()
print()
print(text)
print()
print()
print()
print()
print()
validate_streaming(JSON_PATH.open("rb"))
