local p = game.player;
local sigs = {};
local quals = {};
local function add(p, typ)
    for name, _ in pairs(p) do
        sigs[#sigs + 1] = string.format('    { "type": "%s", "name": "%s" }', typ, name)
    end
end;

for name, quality in pairs(prototypes.quality) do
    quals[#quals + 1] = '"' .. name .. '"'
end

add(prototypes.item, "item");
add(prototypes.fluid, "fluid");
add(prototypes.recipe, "recipe");
add(prototypes.entity, "entity");
add(prototypes.space_location, "space-location");
add(prototypes.asteroid_chunk, "asteroid-chunk");
add(prototypes.quality, "quality");
add(prototypes.virtual_signal, "virtual");

script.on_event(
    defines.events.on_gui_click,
    function(e)
        if e.element and e.element.valid and e.element.name == "custom_signals_copy_window_close" then
            local f = e.element.parent;
            f.destroy();
            script.on_event(defines.events.on_gui_click, nil)
        end
    end
);

if p.gui.screen.custom_signals_copy_window then
    p.gui.screen.custom_signals_copy_window.destroy()
else
    local f = p.gui.screen.add{type="frame", name="custom_signals_copy_window", caption="Signals List", direction="vertical"};
    f.auto_center = true;
    f.add{type="label", caption="Select the signals below and press Ctrl+C to copy it."};

    local text = string.format(
        '{\n    "signals": [\n    %s\n    ],\n    "qualities": [%s]\n}',
        table.concat(sigs, ",\n    "),
        table.concat(quals, ", ")
    );
    local t = f.add{type="text-box", name="bp_text", text=text};
    t.style.width = 600;
    t.style.height = 600;
    t.read_only = true;
    t.selectable = true;
    p.opened = f;
    t.focus();
    t.select_all()
    f.add{type="button", name="custom_signals_copy_window_close", caption="Done"};
    p.opened = f
end
