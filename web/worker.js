import init, { run_blueprint, set_progress_callback } from "../pkg/giftorio_wasm.js";

self.wasmYield = function () {
	return new Promise(resolve => setTimeout(resolve, 0));
};

async function run() {
	await init();

	set_progress_callback((percentage, status) => {
		// Post progress updates to the main thread.
		postMessage({ progress: { percentage, status } });
	});

	addEventListener("message", async message => {
		const { name, imageData, imageType, targetFps, maxSize, useDLC, substationQuality, grayscaleBits, resamplingFilter } = message.data;

		try {
			const blueprint = await run_blueprint(
				name,
				imageData,
				imageType,
				useDLC,
				targetFps,
				maxSize,
				substationQuality,
				grayscaleBits,
				resamplingFilter,
				function onGroupReady(data) {
					postMessage({ blueprint: data });
				},
			);
			postMessage({ blueprint: blueprint });
		} catch (e) {
			console.error("Error generating blueprint:", e);
			postMessage({ error: e.toString() });
		}
	});
}

run();
