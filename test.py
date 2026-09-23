import base64, zlib, json

try:
    import pyperclip
except ImportError:
    print("Install pyperclip with `pip install pyperclip`")
    raise


def decode_blueprint(s: str) -> dict:
    if (s := "".join(s.split())).startswith("0"):
        s = s[1:]
    return zlib.decompress(base64.b64decode(s))


print(json.dumps(json.loads(decode_blueprint(pyperclip.paste())), indent=2))
