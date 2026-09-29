export function formatDuration(totalMs: number): string {
	const totalS = Math.floor(totalMs / 1000);

	const h = Math.floor(totalS / 3600).toString();
	const m = Math.floor((totalS % 3600) / 60).toString();
	const s = (totalS % 60).toString().padStart(2, "0");
	const ms = (totalMs % 1000).toString().padStart(3, "0");

	return `${h.padStart(2, "0")}:${m.padStart(2, "0")}:${s.padStart(2, "0")}.${ms.padStart(3, "0")}`;
}
export function formatFileSize(bytes: number) {
	if (bytes === 0) return "0 Bytes";

	const units = ["Bytes", "KB", "MB", "GB", "TB"];
	const i = Math.floor(Math.log(bytes) / Math.log(1024));
	const value = bytes / 1024 ** i;

	return `${+value.toFixed(2)} ${units[i]}`;
}
