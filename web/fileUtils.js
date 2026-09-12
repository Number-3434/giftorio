/** @type {Promise<IDBDatabase>} */
const dbPromise = new Promise((resolve, reject) => {
	const request = indexedDB.open("app-storage", 1);

	request.onupgradeneeded = () => request.result.createObjectStore("files");
	request.onsuccess = () => resolve(request.result);
	request.onerror = () => reject(request.error);
});

export async function saveFileDB(file, key) {
	const db = await dbPromise;

	return new Promise((resolve, reject) => {
		const tx = db.transaction("files", "readwrite");

		tx.objectStore("files").put(file, key);
		tx.oncomplete = () => resolve();
		tx.onerror = () => reject(tx.error);
	});
}

export async function loadFileDB(key) {
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
