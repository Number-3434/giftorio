import _INITIAL_VALUES from "./initialValues.json";
import FORM_ELEMENTS from "./formElements.json";

export { FORM_ELEMENTS };

// Constants
export const DEFAULT_SIGNAL_PRESET = "base-2.0.77";
export const LAST_FILE_KEY = "last-file-user-uploaded";
export const FORM_DATA_KEY = "giftorio-form-data";
export const SHOW_ADVANCED_KEY = "giftorio-form-data-show-advanced";
export const CURR_SIGNAL_PRESET_KEY = "giftorio-curr-preset-key";
export const SIGNAL_PRESETS_KEY = "giftorio-signal-presets";

export interface SignalPreset {
	description: string;
	isDefault?: boolean;
	signalsCSV: string;
	qualitiesCSV: string;
}

export function setInitialValues(values: object) {
	localStorage.setItem(FORM_DATA_KEY, JSON.stringify(values));
}
export const INITIAL_VALUES: Omit<typeof _INITIAL_VALUES, "file"> & { file: File | null } = (() => {
	const prev = localStorage.getItem(FORM_DATA_KEY);

	if (prev) {
		const cached = JSON.parse(prev);
		let changed = false;

		for (const k of Object.keys(cached)) {
			if (!(k in _INITIAL_VALUES)) {
				delete cached[k];
				changed = true;
			}
		}
		for (const [k, v] of Object.entries(_INITIAL_VALUES)) {
			if (!(k in cached)) {
				cached[k] = v;
				changed = true;
			}
		}

		if (changed) {
			setInitialValues(cached);
		}
		return cached;
	}

	setInitialValues(_INITIAL_VALUES);
	return { ..._INITIAL_VALUES };
})();
