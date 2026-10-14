import json
from pathlib import Path
import zlib
import base64


def encode_blueprint(s: str) -> str:
    o = s.encode("utf-8")
    compressed = zlib.compress(o)
    encoded = base64.b64encode(compressed).decode("ascii")
    return f"0{encoded}"


bp = {
    "blueprint": {
        "icons": [{"signal": {"name": "decider-combinator"}, "index": 1}],
        "entities": [
            {
                "entity_number": 1,
                "name": "decider-combinator",
                "position": {"x": -606.5, "y": 900},
                "control_behavior": {
                    "decider_conditions": {
                        "conditions": [
                            {"first_signal": {"type": "virtual", "name": "signal-red"}, "comparator": "\u2260", "compare_type": "and"},
                            {"first_signal": {"type": "virtual", "name": "signal-T"}, "constant": 1, "comparator": "=", "compare_type": "and"},
                            {"first_signal": {"type": "virtual", "name": "signal-red"}, "comparator": "\u2260"},
                            {"first_signal": {"type": "virtual", "name": "signal-T"}, "constant": 2, "comparator": "=", "compare_type": "and"},
                        ],
                        "outputs": [{"signal": {"type": "virtual", "name": "signal-everything"}, "copy_count_from_input": False, "constant": 235}],
                    }
                },
            }
        ],
        "item": "blueprint",
        "version": 562949958467584,
    }
}

conds = []


for i in range(100_000):
    conds.extend([
        {"first_signal": {"type": "virtual", "name": "signal-red"}, "comparator": "\u2260"},
        {"first_signal": {"type": "virtual", "name": "signal-T"}, "constant": i + 3, "comparator": "=", "compare_type": "and"},
    ])

bp["blueprint"]["entities"][0]["control_behavior"]["decider_conditions"]["conditions"] = conds
json_bytes = json.dumps(bp, separators=(",", ":"), indent=2)


Path("output.txt").write_text(encode_blueprint(json_bytes))
