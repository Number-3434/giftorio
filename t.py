import json, base64, pyperclip, zlib

# SETTINGS
MIN_SEPARATION = 0.8 + 1 / 1000
START = 1
END = 50

blueprint_books = []

for blueprint_book_i, is_hex in enumerate([False, True]):
    blueprints = []

    for i in range(START, END + 1):
        count = int(i // MIN_SEPARATION)
        entities = []
        separation = i / count

        for x in range(count):
            for y in range(count):
                entities.append({
                    "entity_number": len(entities) + 1,
                    "name": "land-mine",
                    "position": {
                        "x": (x + ((1.0 if y % 2 == 1 else 0.5) if is_hex else 0.5)) * separation,
                        "y": (y + 0.5) * separation,
                    },
                })

        blueprints.append({
            "blueprint": {
                "snap-to-grid": {"x": i, "y": i},
                "icons": [{"signal": {"name": "land-mine"}, "index": 1}],
                "description": "\n".join([
                    f"[img=item.land-mine] [font=compi]{count}x{count} Landmine{(count != 1) * 's'} in a {i}x{i} grid.[/font]",
                    f"Separation: {separation} tiles.",
                    "",
                ]),
                "entities": entities,
                "item": "blueprint",
                "label": f"[font=var]{i}[/font]x[font=var]{i}[/font] Tile{(i != 1) * 's'}: [font=var]{count}[/font]x[font=var]{count}[/font] [item=land-mine]{' (Hex)' * is_hex}\n\nDensity: [font=var]{100 / (separation**2):.2f}%[/font]",
                "version": 562949958467584,
            }
        })

    blueprint_books.append({
        "blueprint_book": {
            "item": "blueprint-book",
            "icons": [{"signal": {"type": "virtual", "name": "shape-diagonal-cross" if is_hex else "shape-cross"}, "index": 1}],
            "blueprints": blueprints,
            "label": "Hexagonal Layout" if is_hex else "Square Layout",
            "index": blueprint_book_i,
            "description": "Hexagonally packed in square rows, with every other row offset by 1/2 of the spacing." if is_hex else "Squarely packed in rows.",
            "version": 562949958467584,
        }
    })


data = json.dumps({
    "blueprint_book": {
        "item": "blueprint-book",
        "icons": [{"signal": {"name": "land-mine"}, "index": 1}],
        "blueprints": blueprint_books,
        "label": f"Compact Landmines (Seamless): {START}x{START} to {END}x{END} Tile{(END != 1) * 's'}",
        "description": f"Groups of [item=land-mine] landmines packed compactly, ranging from {START}x{START} to {END}x{END} tiles.\n\nMinimum Separation: {MIN_SEPARATION} tiles.",
        "active_index": 0,
        "version": 562949958467584,
    }
})
data = zlib.compress(data.encode("utf-8"), 9)
data = "0" + base64.b64encode(data).decode("utf-8")

pyperclip.copy(data)
