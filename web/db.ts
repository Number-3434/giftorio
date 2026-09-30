import { gunzipSync, gzipSync } from "fflate";
import { SignalPreset } from "./data";

const APP_STORAGE_KEY = "app-storage";
const FILE_STORAGE_KEY = "file-storage";
const SIGNAL_PRESETS_STORAGE_KEY = "signal-data-presets";

// Note: We need to explicitly use the `import()` syntax 4 times so vite can bundle and resolve the URLs
const PRESET_MAP = {
	"base-2.0.77": {
		getSignalsUrl: () => import("./assets/data/presets/base-2.0.77/signals.csv?url"),
		getQualitiesUrl: () => import("./assets/data/presets/base-2.0.77/qualities.csv?url"),
		description: "Signals from the base game (v2.0.77).",
	},
	"space-age-2.0.77": {
		getSignalsUrl: () => import("./assets/data/presets/space-age-2.0.77/signals.csv?url"),
		getQualitiesUrl: () => import("./assets/data/presets/space-age-2.0.77/qualities.csv?url"),
		description: "Signals from the Space Age DLC (v2.0.77).",
	},
};
export const DEFAULT_SIGNAL_PRESET_KEYS = Object.keys(PRESET_MAP);

const fileDBPromise: Promise<IDBDatabase> = new Promise((resolve, reject) => {
	const req = indexedDB.open(APP_STORAGE_KEY, 1);
	req.onupgradeneeded = () => {
		const { result } = req;
		result.createObjectStore(FILE_STORAGE_KEY);
		result.createObjectStore(SIGNAL_PRESETS_STORAGE_KEY);
	};
	req.onsuccess = () => resolve(req.result);
	req.onerror = () => reject(req.error);
});

export async function saveFileDB(file: File | null, key: string): Promise<void> {
	const db = await fileDBPromise;
	return new Promise((resolve, reject) => {
		const tx = db.transaction(FILE_STORAGE_KEY, "readwrite");
		tx.objectStore(FILE_STORAGE_KEY).put(file, key);
		tx.oncomplete = () => resolve();
		tx.onerror = () => reject(tx.error);
	});
}

export async function loadFileDB(key: string): Promise<File | null> {
	const db = await fileDBPromise;
	return new Promise((resolve, reject) => {
		try {
			const req = db.transaction(FILE_STORAGE_KEY, "readonly").objectStore(FILE_STORAGE_KEY).get(key);
			req.onsuccess = () => resolve(req.result ?? null);
			req.onerror = () => reject(req.error);
		} catch (e) {
			return resolve(null);
		}
	});
}

export async function getSignalPresetNames(): Promise<string[]> {
	const db = await fileDBPromise;
	return new Promise((resolve, reject) => {
		const tx = db.transaction(SIGNAL_PRESETS_STORAGE_KEY, "readonly");
		const req = tx.objectStore(SIGNAL_PRESETS_STORAGE_KEY).getAllKeys();
		req.onsuccess = () => resolve(req.result as string[]);
		req.onerror = () => reject(tx.error);
	});
}
export async function addSignalPreset(name: string, preset: SignalPreset): Promise<void> {
	const db = await fileDBPromise;
	return new Promise((resolve, reject) => {
		const tx = db.transaction(SIGNAL_PRESETS_STORAGE_KEY, "readwrite");
		const compressed = gzipSync(new TextEncoder().encode(JSON.stringify(preset)), { level: 9 });
		const req = tx.objectStore(SIGNAL_PRESETS_STORAGE_KEY).put(compressed, name);
		tx.oncomplete = () => resolve();
		tx.onerror = () => reject(req.error);
	});
}
export async function getSignalPreset(name: string): Promise<SignalPreset | null> {
	if (name in PRESET_MAP) {
		const preset = PRESET_MAP[name as keyof typeof PRESET_MAP];
		return {
			description: preset.description,
			signalsCSV: await preset
				.getSignalsUrl()
				.then((v) => fetch(v.default))
				.then((v) => v.arrayBuffer())
				.then((v) => new Uint8Array(v)),
			qualitiesCSV: await preset
				.getQualitiesUrl()
				.then((v) => fetch(v.default))
				.then((v) => v.arrayBuffer())
				.then((v) => new Uint8Array(v)),
		} satisfies SignalPreset;
	}

	const db = await fileDBPromise;
	return new Promise((resolve, reject) => {
		try {
			const tx = db.transaction(SIGNAL_PRESETS_STORAGE_KEY, "readonly");
			const req = tx.objectStore(SIGNAL_PRESETS_STORAGE_KEY).get(name);
			req.onsuccess = () => resolve(JSON.parse(new TextDecoder().decode(gunzipSync(req.result))));
			req.onerror = () => reject(req.error);
		} catch (e) {
			return resolve(null);
		}
	});
}
