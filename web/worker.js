import init, { run_blueprint, set_progress_callback } from "../pkg/giftorio_wasm.js";

async function run() {
	await init();

	set_progress_callback((percentage, status) => {
		postMessage({ progress: { percentage, status } });
	});

	const pendingWrites = new Map();
	let nextWriteId = 0;

	addEventListener("message", async event => {
		if (event.data.type === "chunkWritten") {
			const pending = pendingWrites.get(event.data.id);
			if (!pending) return;

			pendingWrites.delete(event.data.id);
			event.data.error ? pending.reject(new Error(event.data.error)) : pending.resolve();
		} else if (event.data.type === "generate") {
			const { name, imageData, imageType, targetFps, maxSize, useDLC, substationQuality, grayscaleBits, resamplingFilter } =
				event.data;

			try {
				postMessage({ type: "start", filename: "blueprint.json" });

				await run_blueprint(
					name,
					imageData,
					imageType,
					useDLC,
					targetFps,
					maxSize,
					substationQuality,
					grayscaleBits,
					resamplingFilter,
					data => postMessage({ blueprint: data }),
					chunk =>
						new Promise((resolve, reject) => {
							const id = nextWriteId++;
							pendingWrites.set(id, { resolve, reject });
							postMessage({ chunk: { id, data: chunk } }, [chunk.buffer]);
						}),
				);
				postMessage({ type: "done" });
			} catch (e) {
				postMessage({ error: e?.toString() ?? String(e) });
			}
		}
	});
}

run();
