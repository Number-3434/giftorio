import { createSignal, For } from "solid-js";
import { FormElementValue } from "../types";
import { Tooltip } from "./Tooltip";

export type SelectOption = string | { name: string; tooltip?: string };
export interface FactorioSelectProps {
	formArgs: FormElementValue;
	initialValue: string;
	key: string;
	name: string;
	onChange(value: string): void;
	tooltip?: string;
	options: [string, SelectOption][];
	splash?: string;
	ref(el: HTMLSelectElement): void;
}

export function FactorioSelect(props: FactorioSelectProps) {
	const { formArgs, initialValue, key, name, onChange, options, tooltip, splash, ref } = props;
	const [value, setValue] = createSignal(initialValue);

	function handleChange(e: Event) {
		const target = e.currentTarget as HTMLSelectElement;
		setValue(target.value);
		onChange(target.value);
	}

	return (
		<div
			class="mt-1 mb-1 flex items-center justify-between factorio-form-element factorio-select-container"
			aria-disabled={formArgs.disabled}
		>
			<label class="block text-white-500" for={key}>
				{name}
				<Tooltip name={name} tooltip={tooltip} splash={splash} options={options} />
			</label>
			<select ref={ref} id={key} name={key} value={value()} onChange={handleChange}>
				<For each={options}>{([k, v]) => <option value={k}>{(v as any).name ?? v}</option>}</For>
			</select>
		</div>
	);
}
