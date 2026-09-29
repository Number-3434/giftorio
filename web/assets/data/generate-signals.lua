local p = game.player;
local t = {};
local function add(p, typ)
    for n, _ in pairs(p) do
        t[#t + 1] = typ .. "," .. n
    end
end;

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

    local t = f.add{type="text-box", name="bp_text", text=table.concat(t,"\n")};
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
