import { createEffect, createResource, createSignal, For, onMount, Show, Suspense } from "solid-js";
import { Tooltip } from "./Tooltip";
import { DEFAULT_SIGNAL_PRESET_KEYS, getSignalPreset, getSignalPresetNames } from "../db";
import { CURR_SIGNAL_PRESET_KEY, DEFAULT_SIGNAL_PRESET, SignalPreset } from "../data";

export interface SignalPresetSelectProps {
	ref(el: any): void;
	setToast(v: { show: boolean; message: string; isError: boolean }): void;
}

/// Gets the command to run inside of Factorio to generate signals
async function fetchFactorioSignalCommand(): Promise<string> {
	return await import("../assets/data/generate-signals.lua?url") //
		.then((v) => fetch(v.default))
		.then((v) => v.text());
}
type ExtractionStage = "copy-command" | "paste-signals" | "metadata" | "done";

export function SignalPresetSelect(props: SignalPresetSelectProps) {
	const PREFIX = "signal-preset-";

	const { ref, setToast } = props;
	const [signalPresetKeys, setPresetKeys] = createSignal<string[]>();
	const [currPresetKey, setCurrPresetKey] = createSignal<string | null>(localStorage.getItem(CURR_SIGNAL_PRESET_KEY));
	const [currPreset, setCurrPreset] = createSignal<SignalPreset | null>(null);
	const [factorioSignalCommand] = createResource(fetchFactorioSignalCommand);
	const [extractionStage, setExtractionStage] = createSignal<ExtractionStage | null>(null);

	function textAreaRef(el: HTMLTextAreaElement) {
		onMount(() => {
			el.focus();
			el.select();
		});
	}
	onMount(() => {
		getSignalPresetNames().then(setPresetKeys);

		if (!currPresetKey()) {
			localStorage.setItem(CURR_SIGNAL_PRESET_KEY, DEFAULT_SIGNAL_PRESET);
			setCurrPresetKey(DEFAULT_SIGNAL_PRESET);
		}
	});

	createEffect(() => {
		const key = currPresetKey();
		if (!key) return;

		getSignalPreset(key).then((v) => {
			console.log(v);
			setCurrPreset(v);
		});
	});

	function handleSignalExtraction(evt: Event) {
		evt.preventDefault();
		setExtractionStage("paste-signals");
	}

	return (
		<div class="mt-1 mb-1 flex items-center justify-between factorio-form-element factorio-select-container">
			<label class="text-white-500" for="maxsize">
				Signal Preset
				<Tooltip
					tooltip={"A preset list of all available signals." + "\n\n" + `Current preset: ${currPresetKey()}`}
					name="Signal Preset"
					splash={currPreset()?.description}
				/>
			</label>
			<select
				ref={ref}
				id="signalPreset"
				name="signalPreset"
				value={PREFIX + currPresetKey()}
				onChange={(e) => {
					const presetKey = e.currentTarget.value.replace(PREFIX, "");
					if (presetKey === "add-preset") {
						setExtractionStage("copy-command");
						return;
						const signalsCSV = prompt(
							[
								`/c ${script}`,
								"\n\n",
								"Copy the above Lua code, and paste it into the Factorio console.",
								"Note: you may need to enable Editor mode.\n\nThen enter the signals CSV:",
							].join(" "),
						);

						const newKey = prompt("Enter a name for the new preset:");
						if (!newKey) return;

						return;
					} else {
						setCurrPresetKey(presetKey);
					}
				}}
			>
				<For each={[...DEFAULT_SIGNAL_PRESET_KEYS, ...(signalPresetKeys() || [])]}>
					{(k) => <option value={PREFIX + k}>{k}</option>}
				</For>
				<option value="add-preset">New preset...</option>
			</select>

			<Show when={extractionStage()}>
				<div class="backdrop flex flex-col items-center justify-center">
					<Show when={extractionStage() === "copy-command"}>
						<div class="panel popup max-h-md" style={{ "max-width": "50vw", height: "75vh" }}>
							<div class="font-semibold mb-5 text-tan-500 text-xl">Signal and Quality Extraction: Command Copy</div>
							<div class="text-white-500 text-sm">
								<div>
									This tool extracts all available signals and qualities in your version of Factorio, including signals /
									qualities provided by external mods.
								</div>
								<br />
								<div class="font-semibold">Steps:</div>
								<div>1. Create a new empty save file. Enable all mods you wish to extract signals and qualities from.</div>
								<div>2. Open the Factorio console.</div>
								<div>3. Copy-and-paste the command below into the Factorio console, and hit Enter.</div>
								<div>
									4. A pop-up will be displayed prompting you to copy the resulting data. Press <code>Ctrl+C</code> to
									copy the list.
								</div>
								<div>5. Press Next to continue, after the signals are in your clipboard.</div>
							</div>
							<div class="my-10" />
							<Suspense fallback="Loading script...">
								<textarea
									class="w-full h-full rounded p-3 text-black text-s"
									ref={textAreaRef} // to auto-select content
									readonly
									style={{ background: "#f0dab4", "max-height": "30vh", "overflow-y": "auto" }}
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
								<button class="button button-green-right" onClick={handleSignalExtraction}>
									Next
								</button>
							</div>
						</div>
					</Show>
					<Show when={extractionStage() === "paste-signals"}>
						<div class="panel popup min-h-md" style={{ "max-width": "50vw" }}>
							<div class="font-semibold mb-5 text-tan-500 text-xl">Signal and Quality Extraction: Paste Signals</div>
							<div class="text-white-500 text-sm">
								<div>Please paste the signals below.</div>
							</div>
							<div class="my-10" />
							<textarea
								class="w-full rounded p-3 text-black text-s"
								style={{ background: "#f0dab4", "max-height": "30vh", "overflow-y": "auto" }}
								onInput={(e) => {
									console.log(e.currentTarget.value);
								}}
							/>
							<div class="my-10" />
							<div class="flex justify-between w-full">
								<button class="button bg-gray-100 px-4" type="button" onClick={() => setExtractionStage("copy-command")}>
									Back
								</button>
								<div />
								<button class="button button-green-right" onClick={(e) => {}}>
									Next
								</button>
							</div>
						</div>
					</Show>
				</div>
			</Show>
		</div>
	);
}
