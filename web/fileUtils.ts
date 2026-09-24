const dbPromise: Promise<IDBDatabase> = new Promise((resolve, reject) => {
	const req = indexedDB.open("app-storage", 1);
	req.onupgradeneeded = () => req.result.createObjectStore("files");
	req.onsuccess = () => resolve(req.result);
	req.onerror = () => reject(req.error);
});

export async function saveFileDB(file: File, key: string): Promise<void> {
	const db = await dbPromise;
	return new Promise<void>((resolve, reject) => {
		const tx = db.transaction("files", "readwrite");
		tx.objectStore("files").put(file, key);
		tx.oncomplete = () => resolve();
		tx.onerror = () => reject(tx.error);
	});
}

export async function loadFileDB(key: string): Promise<File | null> {
	const db = await dbPromise;

	return new Promise((resolve, reject) => {
		try {
			const tx = db.transaction("files", "readonly");
			const req = tx.objectStore("files").get(key);
			req.onsuccess = () => resolve(req.result ?? null);
			req.onerror = () => reject(req.error);
		} catch (e) {
			return resolve(null);
		}
	});
}
