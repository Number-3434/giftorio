import { createEffect, createResource, createSignal, For, onMount, Show, Suspense } from "solid-js";
import { createStore } from "solid-js/store";
import { CURR_SIGNAL_PRESET_KEY, SignalData, SignalPreset, validateSignalData } from "../data";
import { addSignalPreset, DEFAULT_SIGNAL_PRESET_KEYS, getSignalPreset, getSignalPresetNames, removeSignalPreset } from "../db";
import { useErrM, ValueOf } from "../utils";
import { EventListener } from "./EventListener";
import { KeyboardListener } from "./KeyboardListener";
import { Tooltip } from "./Tooltip";

export const SIGNAL_SELECTOR_KEYS = Object.freeze(["f", "s", "t"] as const);
export type TimingSignals = Record<(typeof SIGNAL_SELECTOR_KEYS)[number], SignalData["signals"][number] & { quality: string }>;
export interface SignalPresetSelectProps {
  setSignalPreset(v: SignalPreset): void;
  setTimingSignals(v: TimingSignals): void;
  showToast(duration: number, v: { show?: boolean; message: string; isError?: boolean }): void;
}

type ExtractionStage = "copyCommand" | "pasteSignals" | "metadata" | "done";

export function SignalPresetSelect({ setSignalPreset, setTimingSignals, showToast }: SignalPresetSelectProps) {
  const PREFIX = "signal-preset-";
  const DEFAULT_FORM_DATA = { name: "", description: "" };

  const [currPresetKey, setCurrPresetKey] = createSignal<string | null>(null);
  const [currPreset, { mutate: setCurrPreset }] = createResource(currPresetKey, async (k) => (k ? await getSignalPreset(k) : null));
  const [currSignalData, setCurrSignalData] = createSignal<SignalData | null>(null); // Current data we're editing
  const [currTimingSignals, setCurrTimingSignals] = createSignal<TimingSignals | null>(null);
  const [extractionStage, setExtractionStage] = createSignal<ExtractionStage | null>(null);
  const [formData, setFormData] = createStore(DEFAULT_FORM_DATA);
  const [codeCommand] = createResource(async (): Promise<string> => {
    const url = await import("../assets/data/generate-signals.lua?url"); // Note: keep this dyanmic import explicit so Vite knows to bundle it
    return fetch(url.default).then((v) => v.text());
  });
  const [signalPresetKeys, { refetch: refetchPresetKeys }] = createResource(getSignalPresetNames);
  let refDialogDelete: HTMLDialogElement = null!;

  createEffect(() => {
    if (extractionStage()) return;
    setFormData(DEFAULT_FORM_DATA);
    setCurrSignalData(null);
  });

  onMount(() => setCurrPresetKey(localStorage.getItem(CURR_SIGNAL_PRESET_KEY) ?? DEFAULT_SIGNAL_PRESET_KEYS[0]));

  createEffect(() => currPresetKey() && localStorage.setItem(CURR_SIGNAL_PRESET_KEY, currPresetKey()!));
  createEffect(() => setSignalPreset(currPreset()!));
  createEffect(() => setTimingSignals(currTimingSignals()!));

  return (
    <>
      <dialog class="panel fixed max-w-100 left-1/2 top-1/2 -translate-x-1/2 -translate-y-1/2 backdrop:bg-black/60" ref={refDialogDelete}>
        <div class="flex flex-col items-center justify-center">
          <h2 class="text-tan-500">Delete configuration?</h2>
          <p class="text-white text-center ">This will permanently delete the configuration "{currPresetKey()}". This cannot be undone.</p>
          <hr class="my-5" />
          <div class="flex justify-between w-full">
            <button class="button" type="button" onClick={() => refDialogDelete.close()}>
              Cancel
            </button>
            <button
              class="button button-red min-w-[128px]"
              type="button"
              onClick={() => {
                removeSignalPreset(currPresetKey()!)
                  .then(refetchPresetKeys)
                  .then(() => {
                    showToast(3000, { message: `Preset "${currPresetKey()}" removed!` });
                    refDialogDelete.close();
                    setCurrPresetKey(DEFAULT_SIGNAL_PRESET_KEYS[0]);
                  });
              }}
            >
              Delete
            </button>
          </div>
        </div>
      </dialog>
      <div class="mt-5 mb-1 flex flex-col items-start gap-1 justify-between w-full">
        <div class="flex items-center factorio-form-element justify-between w-full">
          <label class="text-white-500" for="maxsize">
            Signal Preset
            <Show when={currPreset()}>
              <Tooltip
                tooltip={<i>{currPreset()!.description}</i>}
                name={`Signal Preset: ${currPresetKey()}`}
                splash="Signal presets define the signals and qualities available for use in the blueprint."
              />
            </Show>
          </label>
          <Show
            when={currPreset()}
            fallback={
              <select>
                <option>Loading...</option>
              </select>
            }
          >
            <div class="flex gap-1">
              <div class="flex factorio-select-container">
                <select
                  id="signalPreset"
                  name="signalPreset"
                  value={PREFIX + currPresetKey()}
                  onChange={(e) => {
                    const presetKey = e.currentTarget.value.replace(PREFIX, "");
                    if (presetKey === "add-preset") {
                      setExtractionStage("copyCommand");
                    } else {
                      setCurrPresetKey(presetKey);
                    }
                  }}
                  title={currPreset()?.description}
                >
                  <For each={DEFAULT_SIGNAL_PRESET_KEYS}>{(k) => <option value={PREFIX + k}>{k}</option>}</For>
                  <For each={signalPresetKeys()}>{(k) => <option value={PREFIX + k}>{k}</option>}</For>
                  <option class="italic" value="add-preset" title="Add a new preset">
                    New preset...
                  </option>
                </select>
              </div>
              <button
                class="button-red w-8 h-8 text-xl font-bold"
                type="button"
                disabled={DEFAULT_SIGNAL_PRESET_KEYS.includes(currPresetKey() ?? "")}
                onClick={() => refDialogDelete.showModal()}
                title={
                  DEFAULT_SIGNAL_PRESET_KEYS.includes(currPresetKey() ?? "")
                    ? "Cannot remove default presets"
                    : `Remove the currently selected preset "${currPresetKey()}"`
                }
              >
                🞫
              </button>
            </div>
          </Show>
        </div>

        <Show when={currPreset()}>
          <SignalSelectors
            values={{
              f: { type: "virtual", name: "signal-F", quality: "normal" },
              s: { type: "virtual", name: "signal-B", quality: "normal" },
              t: { type: "virtual", name: "signal-T", quality: "normal" },
            }}
            signalData={currPreset()!}
            onChange={(v) => setCurrTimingSignals(v)}
          />
        </Show>

        <Show when={extractionStage()}>
          <div class="backdrop flex flex-col items-center justify-center">
            <div
              class="panel flex flex-col popup min-h-md h-[75vh] w-[50vw]"
              onSubmit={(e) => {
                e.preventDefault();
                const STAGES = { copyCommand: "pasteSignals", pasteSignals: "metadata", metadata: null } as const;
                if ((extractionStage() as any) in STAGES) {
                  setExtractionStage(STAGES[extractionStage()! as keyof typeof STAGES]);
                }
              }}
            >
              <KeyboardListener keys={["Escape"]} onKeyDown={() => setExtractionStage(null)} />
              <Show when={extractionStage() === "copyCommand"}>
                <EventListener event="copy" handler={() => showToast(3000, { message: "Copied to clipboard!" })} />
                <KeyboardListener keys={["Enter", "e", "E"]} onKeyDown={() => setExtractionStage("pasteSignals")} />
                <div class="font-semibold mb-5 text-tan-500 text-xl">Create Signal Preset</div>
                <div class="text-white-500">
                  <div>
                    This tool extracts all signals and qualities available in a Factorio save file, including those provided by external
                    mods. Use this tool for compatibility with different versions of Factorio, or to utilise extra signals / qualities
                    provided by mods.
                  </div>
                  <br />
                  <div class="font-semibold">Steps:</div>
                  <div>1. Create a new empty save file. Enable all mods you wish to extract signals and qualities from.</div>
                  <div>2. Open the Factorio console.</div>
                  <div>
                    3. Copy-and-paste the command below into the Factorio console, and hit <code>Enter</code>.
                  </div>
                  <div>
                    4. A pop-up will be displayed prompting you to copy the data. Use <code>Ctrl+C</code> to copy the list.
                  </div>
                  <div>5. Go back to the GIFtorio, and press "Next" to continue.</div>
                </div>
                <div class="my-5" />
                <Suspense fallback="Loading script...">
                  <textarea
                    class="w-full h-full"
                    ref={(e) => onMount(() => (e.focus(), e.select()))}
                    readonly
                    value={`/c ${codeCommand()}`}
                  />
                </Suspense>
                <div class="my-10" />
                <div class="flex justify-between w-full">
                  <button
                    class="button bg-gray-100 px-4"
                    type="button"
                    onClick={(evt) => {
                      evt.preventDefault();
                      navigator.clipboard.writeText(`/c ${codeCommand()}`).then(() => showToast(3000, { message: "Copied to clipboard!" }));
                    }}
                  >
                    Copy
                  </button>
                  <div />
                  <button class="button button-green-right">Next</button>
                </div>
              </Show>
              <Show when={extractionStage() === "pasteSignals"}>
                <div class="font-semibold mb-5 text-tan-500 text-xl">Signal and Quality Extraction: Paste Signals</div>
                <div class="text-white-500 text-sm">
                  <div>Please paste the signals below.</div>
                </div>
                <div class="my-5" />
                <textarea
                  class="w-full h-full"
                  required
                  onChange={(evt) => {
                    evt.preventDefault();

                    const errM = useErrM(evt.currentTarget);
                    try {
                      const parsed = JSON.parse(evt.currentTarget.value);
                      if (validateSignalData(parsed)) {
                        setCurrSignalData(parsed);
                      } else {
                        errM.report("JSON did not match expected format");
                        showToast(3000, { message: "JSON did not match expected format", isError: true });
                      }
                    } catch (e) {
                      errM.report("Invalid JSON");
                      showToast(3000, { message: "Invalid JSON", isError: true });
                    }

                    if (errM.test()) {
                      const { signals, qualities } = currSignalData()!;
                      showToast(3000, { message: `Successfully imported ${signals.length} signals and ${qualities.length} qualities!` });
                    }
                  }}
                />
                <div class="my-10" />
                <div class="flex justify-between w-full">
                  <button class="button bg-gray-100 px-4" type="button" onClick={() => setExtractionStage("copyCommand")}>
                    Back
                  </button>
                  <div />
                  <button class="button button-green-right" onSubmit={(e) => e.preventDefault()}>
                    Next
                  </button>
                </div>
              </Show>
              <Show when={extractionStage() === "metadata"}>
                <div class="font-semibold mb-5 text-tan-500 text-xl">Signal and Quality Extraction: Configuration</div>
                <div class="text-white-500 text-sm">
                  <div>Configure settings for this signal preset.</div>
                </div>
                <div class="my-5" />
                <div class="flex flex-col gap-3">
                  <div class="flex flex-row justify-between gap-5">
                    <label class="text-white-500" for="presetName">
                      Name
                      <Tooltip name="Preset Name" splash="Must be unique!" />
                    </label>
                    <input
                      class="bg-gray-100 focus:bg-tan-500 flex-grow px-3 py-2 border rounded focus:outline-none focus:ring"
                      type="text"
                      id="presetName"
                      name="presetName"
                      value={formData.name}
                      onChange={(e) => setFormData("name", e.currentTarget.value)}
                    />
                  </div>
                  <div class="flex flex-row justify-between gap-5">
                    <label class="text-white-500" for="presetName">
                      Description
                      <Tooltip name="Description" tooltip="(Optional) A description of the preset." />
                    </label>
                    <input
                      class="bg-gray-100 focus:bg-tan-500 flex-grow px-3 py-2 border rounded focus:outline-none focus:ring"
                      type="text"
                      id="presetName"
                      name="presetName"
                      value={formData.description}
                      onChange={(e) => setFormData("description", e.currentTarget.value)}
                    />
                  </div>
                </div>
                <div class="h-full" />
                <div class="my-10" />
                <div class="flex justify-between w-full">
                  <button class="button bg-gray-100 px-4" type="button" onClick={() => setExtractionStage("pasteSignals")}>
                    Back
                  </button>
                  <div />
                  <button
                    class="button button-green-right"
                    onClick={() => {
                      const key = formData.name;
                      const preset = { ...(currSignalData() as SignalData), description: formData.description };

                      addSignalPreset(key, preset)
                        .then(refetchPresetKeys)
                        .then(() => {
                          setCurrPresetKey(key);
                          setCurrPreset(preset);
                          setExtractionStage(null);
                        });
                    }}
                  >
                    Import
                  </button>
                </div>
              </Show>
            </div>
          </div>
        </Show>
      </div>
    </>
  );
}

interface SignalSelectorProps {
  values: TimingSignals;
  signalData: SignalData;
  onChange(v: TimingSignals): void;
}
function SignalSelectors(props: SignalSelectorProps) {
  const [currValues, setCurrValues] = createSignal(props.values);
  const [allowedTypeMap] = createResource(
    () => props.signalData.signals,
    (sigs) => {
      const typeMap = new Map<string, string[]>();
      for (const s of sigs) {
        typeMap.set(s.type, typeMap.get(s.type) ?? []);
        typeMap.get(s.type)!.push(s.name);
      }
      return typeMap;
    },
  );

  createEffect(() => props.onChange(currValues()));

  function SignalSelector(props: {
    allowedTypeMap: Map<string, string[]>;
    onChange(v: ValueOf<TimingSignals>): void;
    otherValues: ValueOf<TimingSignals>[];
    signalData: SignalData;
    value: ValueOf<TimingSignals>;
  }) {
    const [currValue, setCurrValue] = createSignal(props.value);

    createEffect(() => props.onChange(currValue()));

    function hasSignal(signal: ValueOf<TimingSignals>) {
      const { type, name, quality } = signal;
      return props.otherValues.find((v) => v.type === type && v.name === name && v.quality === quality);
    }

    return (
      <div class="flex">
        <input
          class="py-1 max-w-50 text-sm font-semibold"
          list="signals"
          type="text"
          placeholder="<type>.<name>"
          value={`${currValue().type}.${currValue().name}`}
          onInput={(evt) => {
            evt.preventDefault();

            const el = evt.currentTarget;
            const errM = useErrM(el);
            const parts = el.value.split(".");

            if (parts.length !== 2) {
              errM.report("Invalid format, expected: <type>,<name>");
              return;
            }
            const [type, name] = parts.map((v) => v.trim());

            if (!props.allowedTypeMap.has(type)) {
              errM.report(
                `Signal type "${type}" not found.\n\nAvailable types: ` +
                  [...new Set(props.allowedTypeMap.keys().map((t) => `"${t}"`))].join(", "),
              );
            } else if (!props.allowedTypeMap.get(type)!.includes(name)) {
              errM.report(`Signal "${type}.${name}" does not exist.`);
            } else if (hasSignal({ ...currValue(), type, name })) {
              errM.report(`Duplicate signal "${type}.${name} with quality ${currValue().quality}"`);
            }
            if (!errM.test()) return;

            setCurrValue((prev) => ({ ...prev, name, type }));
          }}
        />
        <input
          class="py-1 max-w-15 text-xs font-semibold"
          list="qualities"
          type="text"
          value={currValue().quality}
          disabled={props.signalData.qualities.length === 0}
          placeholder="Quality"
          onInput={(evt) => {
            evt.preventDefault();

            const el = evt.currentTarget;
            const quality = el.value;
            const errM = useErrM(el);

            if (!props.signalData.qualities.includes(quality)) {
              errM.report(`"${quality}" must be one of: ${props.signalData.qualities.map((q) => `"${q}"`).join(", ")}`);
            } else if (hasSignal({ ...currValue(), quality })) {
              errM.report(`Duplicate signal "${currValue().type}.${currValue().name} with quality ${quality}"`);
            }
            if (!errM.test()) return;

            setCurrValue((prev) => ({ ...prev, quality }));
          }}
        />
      </div>
    );
  }

  return (
    <div class="w-full space-evenly flex flex-col gap-2">
      <Show when={allowedTypeMap()}>
        <datalist id="signals">
          <For each={Array.from(allowedTypeMap()!.entries())}>
            {([k, v]) => <For each={v}>{(v) => <option value={`${k}.${v}`} />}</For>}
          </For>
        </datalist>
        <datalist id="qualities">
          <For each={props.signalData.qualities}>{(v) => <option value={v} />}</For>
        </datalist>
        <For each={SIGNAL_SELECTOR_KEYS}>
          {(key) => (
            <div class="flex items-center factorio-form-element justify-between w-full">
              <label class="text-white-500" for="maxsize">
                Signal {key.toUpperCase()}
                <Tooltip
                  name={`Signal ${key.toUpperCase()}`}
                  tooltip="Used to internally time the blueprint. All timing signals must be unique."
                  splash="Reserved for timing wires; data combinators will not use these signals."
                />
              </label>
              <SignalSelector
                otherValues={(() => {
                  const { [key]: _, ...rest } = currValues();
                  return Object.values(rest);
                })()}
                onChange={(v) => setCurrValues((prev) => ({ ...prev, [key]: v }))}
                value={currValues()[key]}
                signalData={props.signalData}
                allowedTypeMap={allowedTypeMap()!}
              />
            </div>
          )}
        </For>
      </Show>
    </div>
  );
}
