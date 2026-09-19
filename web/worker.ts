import init, { run_blueprint, set_progress_callback } from "../pkg/giftorio_wasm.js";

async function run() {
  await init();

  set_progress_callback((percentage: number, status: string) => postMessage({ progress: { percentage, status } }));

  const pendingWrites = new Map();
  let nextWriteId = 0;

  addEventListener("message", async (event) => {
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
          // Add Factorio verson prefix
          const id = nextWriteId++;
          const data = new TextEncoder().encode("0");
          postMessage({ chunk: { id, data } }, { targetOrigin: "*", transfer: [data.buffer] });
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
    }
  });
}

run();
