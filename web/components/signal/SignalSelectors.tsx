import Tooltip from "@/components/Tooltip";
import { SignalData } from "@/data";
import { useErrM } from "@/utils";
import { createEffect, createResource, createSignal, For, Show } from "solid-js";

export const SIGNAL_SELECTOR_KEYS = Object.freeze(["f", "s", "t"] as const);
export type TimingSignal = SignalData["signals"][number] & { quality: string };
export type TimingSignals = Record<(typeof SIGNAL_SELECTOR_KEYS)[number], TimingSignal>;

export interface SignalSelectorsProps {
  values: TimingSignals;
  signalData: SignalData;
  onChange(v: TimingSignals): void;
}
export default function SignalSelectors(props: SignalSelectorsProps) {
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

interface SignalSelectorProps {
  allowedTypeMap: Map<string, string[]>;
  onChange(v: TimingSignal): void;
  otherValues: TimingSignal[];
  signalData: SignalData;
  value: TimingSignal;
}
function SignalSelector(props: SignalSelectorProps) {
  const [currValue, setCurrValue] = createSignal(props.value);

  createEffect(() => props.onChange(currValue()));

  function hasSignal(signal: TimingSignal) {
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
