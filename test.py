import base64, zlib, json, pyperclip


def decode_blueprint(s: str) -> dict:
    try:
        return json.loads(s)
    except json.decoder.JSONDecodeError:
        pass

    s = "".join(s.split())

    # First character is the Factorio blueprint version.
    if s.startswith("0"):
        s = s[1:]

    compressed = base64.b64decode(s)
    data = zlib.decompress(compressed)

    return json.loads(data)


print(json.dumps(decode_blueprint(pyperclip.paste()), indent=4))
