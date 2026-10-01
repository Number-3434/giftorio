import { createEffect, createResource, createSignal, For, onMount, Show, Suspense } from "solid-js";
import { createStore } from "solid-js/store";
import { CURR_SIGNAL_PRESET_KEY, SignalData, SignalPreset, validateSignalData } from "../data";
import { addSignalPreset, DEFAULT_SIGNAL_PRESET_KEYS, getSignalPreset, getSignalPresetNames } from "../db";
import { binaryInsert, useErrM } from "../utils";
import { KeyboardListener } from "./keyboardListener";
import { Tooltip } from "./Tooltip";

export interface SignalPresetSelectProps {
	onChange(v: SignalPreset): void;
	setToast(v: { show: boolean; message: string; isError: boolean }): void;
}

type ExtractionStage = "copyCommand" | "pasteSignals" | "metadata" | "done";

export function SignalPresetSelect({ onChange, setToast }: SignalPresetSelectProps) {
	const PREFIX = "signal-preset-";
	const DEFAULT_FORM_DATA = { name: "", description: "" };

	const [currPresetKey, setCurrPresetKey] = createSignal<string | null>(null);
	const [currPreset, { mutate: setCurrPreset }] = createResource(currPresetKey, async (key) => {
		if (!key) return null;
		return await getSignalPreset(key);
	});
	const [currSignalData, setCurrSignalData] = createSignal<SignalData | null>(null); // Current data we're editing
	const [extractionStage, setExtractionStage] = createSignal<ExtractionStage | null>(null);
	const [formData, setFormData] = createStore(DEFAULT_FORM_DATA);
	const [factorioSignalCommand] = createResource(async (): Promise<string> => {
		const { default: url } = await import("../assets/data/generate-signals.lua?url"); //
		return fetch(url).then((v) => v.text());
	});
	const [signalPresetKeys, { refetch: refetchPresetKeys }] = createResource(getSignalPresetNames);

	onMount(() => setCurrPresetKey(localStorage.getItem(CURR_SIGNAL_PRESET_KEY)));

	createEffect(() => {
		const key = currPresetKey();
		key && localStorage.setItem(CURR_SIGNAL_PRESET_KEY, key);
	});

	createEffect(() => {
		if (!extractionStage()) {
			setFormData(DEFAULT_FORM_DATA);
			setCurrSignalData(null);
		}
	});

	createEffect(() => {
		onChange(currPreset()!);
	});

	function renderExtractionStage() {
		return (
			<div class="panel flex flex-col popup min-h-md" style={{ height: "75vh", width: "50vw" }}>
				<KeyboardListener keys={["Escape"]} onKeyDown={() => setExtractionStage(null)} />
				<Show when={extractionStage() === "copyCommand"}>
					<div class="font-semibold mb-5 text-tan-500 text-xl">Signal and Quality Extraction: Command Copy</div>
					<div class="text-white-500 text-sm">
						<div>
							This tool extracts all signals and qualities available in a Factorio save file, including signals / qualities
							provided by external mods.
						</div>
						<br />
						<div class="font-semibold">Steps:</div>
						<div>1. Create a new empty save file. Enable all mods you wish to extract signals and qualities from.</div>
						<div>2. Open the Factorio console.</div>
						<div>3. Copy-and-paste the command below into the Factorio console, and hit Enter.</div>
						<div>
							4. A pop-up will be displayed prompting you to copy the resulting data. Press <code>Ctrl+C</code> to copy the
							list.
						</div>
						<div>5. Press Next to continue, after the signals are in your clipboard.</div>
					</div>
					<div class="my-5" />
					<Suspense fallback="Loading script...">
						<textarea
							class="w-full h-full"
							ref={(el) =>
								onMount(() => {
									// auto-select content
									el.focus();
									el.select();
								})
							}
							readonly
							value={`/c ${factorioSignalCommand()}`}
						/>
					</Suspense>
					<div class="my-10" />
					<div class="flex justify-between w-full">
						<button
							class="button bg-gray-100 px-4"
							type="button"
							onClick={(evt) => {
								evt.preventDefault();
								navigator.clipboard.writeText(`/c ${factorioSignalCommand()}`).then(() => {
									setToast({ show: true, message: "Code copied to clipboard!", isError: false });
									setTimeout(() => setToast({ show: false, message: "", isError: false }), 3000);
								});
							}}
						>
							Copy
						</button>
						<div />
						<button
							class="button button-green-right"
							onClick={(evt) => {
								evt.preventDefault();
								setExtractionStage("pasteSignals");
							}}
						>
							Next
						</button>
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
						onChange={(e) => {
							const t = e.currentTarget;
							let errMsg = "";
							try {
								const parsed = JSON.parse(e.currentTarget.value);
								if (!validateSignalData(parsed)) {
									errMsg = "JSON did not match expected format";
								} else {
									setCurrSignalData(parsed);
								}
							} catch (e) {
								errMsg = "Invalid JSON";
							}

							if (errMsg) {
								t.setCustomValidity(errMsg);
								t.setAttribute("aria-invalid", "true");
								t.reportValidity();
							} else {
								t.setAttribute("aria-invalid", "false");
								t.setCustomValidity("");
								t.reportValidity();
								setExtractionStage("metadata");
							}
						}}
					/>
					<div class="my-10" />
					<div class="flex justify-between w-full">
						<button class="button bg-gray-100 px-4" type="button" onClick={() => setExtractionStage("copyCommand")}>
							Back
						</button>
						<div />
						<button class="button button-green-right">Next</button>
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
								<Tooltip
									name="Preset Name"
									tooltip="The name used to identify the preset in the dropdown."
									splash="Must be unique!"
								/>
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
							onClick={(evt) => {
								evt.preventDefault();

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
		);
	}

	return (
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
					<div class="factorio-select-container">
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
							<option
								disabled={DEFAULT_SIGNAL_PRESET_KEYS.includes(currPresetKey() ?? "")}
								class="italic"
								value="remove-preset"
								title={
									DEFAULT_SIGNAL_PRESET_KEYS.includes(currPresetKey() ?? "")
										? "Cannot remove default presets"
										: `Remove the currently selected preset "${currPresetKey()}"`
								}
							>
								Remove current...
							</option>
						</select>
					</div>
				</Show>
			</div>

			<Show when={currPreset()}>
				<SignalSelectors
					values={{
						b: {
							signal: { type: "virtual", name: "signal-B" },
							quality: "normal",
						},
						f: {
							signal: { type: "virtual", name: "signal-F" },
							quality: "normal",
						},
						t: {
							signal: { type: "virtual", name: "signal-T" },
							quality: "normal",
						},
					}}
					signalData={currPreset()!}
				/>
			</Show>

			<Show when={extractionStage()}>
				<form class="backdrop flex flex-col items-center justify-center">{renderExtractionStage()}</form>
			</Show>
		</div>
	);
}

const SIGNAL_SELECTOR_KEYS = ["f", "b", "t"] as const;
type SignalSelectorKey = (typeof SIGNAL_SELECTOR_KEYS)[number];
interface SignalSelectorProps {
	values: Record<SignalSelectorKey, { signal: SignalData["signals"][number]; quality: string }>;
	signalData: SignalData;
}
function SignalSelectors(props: SignalSelectorProps) {
	const [currValues, setCurrValues] = createStore(props.values);
	const [allowedTypeMap] = createResource(
		() => props.signalData.signals,
		(sigs) => {
			const typeMap = new Map<string, string[]>();
			for (const s of sigs) {
				typeMap.set(s.type, typeMap.get(s.type) ?? []);
				if (false) {
					binaryInsert(typeMap.get(s.type)!, s.name, (a, b) => a.localeCompare(b));
				} else {
					typeMap.get(s.type)!.push(s.name);
				}
			}
			return typeMap;
		},
	);

	function SignalSelector(props: {
		allowedTypeMap: Map<string, string[]>;
		onChange(v: SignalSelectorProps["values"][keyof SignalSelectorProps["values"]]): void;
		signalData: SignalData;
		value: SignalSelectorProps["values"][keyof SignalSelectorProps["values"]];
	}) {
		const [currSignal, setCurrSignal] = createSignal(props.value.signal);
		const [currQuality, setCurrQuality] = createSignal(props.value.quality);

		createEffect(() => props.onChange({ signal: currSignal(), quality: currQuality() }));

		return (
			<div class="flex">
				<input
					class="py-1 max-w-40 text-sm font-semibold"
					list="signals"
					type="text"
					value={`${currSignal().type},${currSignal().name}`}
					onChange={(evt) => {
						evt.preventDefault();

						const el = evt.currentTarget;
						const errM = useErrM(el);
						const parts = el.value.split(",");

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
							errM.report(`Signal name "${name}" not found for type "${type}".`);
						}
						if (!errM.test()) return;

						setCurrSignal({ name, type });
					}}
				/>
				<input
					class="py-1 max-w-20 text-sm font-semibold"
					list="qualities"
					type="text"
					value={currQuality()}
					disabled={props.signalData.qualities.length === 0}
					onChange={(evt) => {
						evt.preventDefault();

						const el = evt.currentTarget;
						const qual = el.value;
						const errM = useErrM(el);

						if (!props.signalData.qualities.includes(qual)) {
							errM.report(
								`Quality "${qual}" not found.\n\nAvailable qualities: ${props.signalData.qualities.map((q) => `"${q}"`).join(", ")}`,
							);
						}
						if (!errM.test()) return;

						setCurrQuality(qual);
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
						{([k, v]) => <For each={v}>{(v) => <option value={`${k},${v}`} />}</For>}
					</For>
				</datalist>
				<datalist id="qualities">
					<For each={props.signalData.qualities}>{(v) => <option value={v} />}</For>
				</datalist>
				<For each={SIGNAL_SELECTOR_KEYS}>
					{(key) => (
						<form class="flex items-center factorio-form-element justify-between w-full">
							<label class="text-white-500" for="maxsize">
								Signal "{key}"
								<Tooltip tooltip="asd" name={`Timing Signal: ${key}`} />
							</label>
							<SignalSelector
								onChange={(v) => setCurrValues(key, v)}
								value={currValues[key]}
								signalData={props.signalData}
								allowedTypeMap={allowedTypeMap()!}
							/>
						</form>
					)}
				</For>
			</Show>
		</div>
	);
}
