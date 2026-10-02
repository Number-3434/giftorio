import init, { init as prepare_wasm, run_blueprint, set_progress_callback, set_signal_data } from "../pkg/giftorio_wasm.js";
import { SignalPreset } from "./data.jsx";

const wasmReady = (async () => {
  await init();
  prepare_wasm();
  set_progress_callback((percentage: number, status: string) => postMessage({ progress: { percentage, status } }));
})();

const pendingWrites = new Map();
let nextWriteId = 0;
let setSignalDataPromise: Promise<void> | null = null;

addEventListener("message", async (event) => {
  await wasmReady;

  if (event.data.type === "chunkWritten") {
    const pending = pendingWrites.get(event.data.id);
    if (!pending) return;

    pendingWrites.delete(event.data.id);
    event.data.error ? pending.reject(new Error(event.data.error)) : pending.resolve();
  } else if (event.data.generate) {
    const { imageData, args } = event.data.generate;
    const { outputFormat } = args;

    try {
      if (outputFormat === "blueprint") {
        postMessage({ type: "start", filename: "blueprint.bp" });
      } else {
        postMessage({ type: "start", filename: "blueprint.json" });
      }
      const blueprintMetadata = await run_blueprint(
        args,
        imageData,
        (data: Uint8Array) =>
          new Promise((resolve, reject) => {
            const id = nextWriteId++;
            pendingWrites.set(id, { resolve, reject });
            postMessage({ chunk: { id, data } }, { targetOrigin: "*", transfer: [data.buffer] });
          }),
      );
      postMessage({ blueprintMetadata });
      postMessage({ type: "done" });
    } catch (e) {
      postMessage({ error: e?.toString() ?? String(e) });
    }
  } else if (event.data.signalPreset) {
    const preset: SignalPreset = event.data.signalPreset;
    set_signal_data(new TextEncoder().encode(JSON.stringify(preset))).then(() => {
      console.log("Yay");
    });
  }
});
