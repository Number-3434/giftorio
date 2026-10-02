/* @refresh reload */
import { render } from "solid-js/web";
import streamSaver from "streamsaver";
import App from "./App";
import "./index.css";

streamSaver.mitm = "https://jimmywarting.github.io/StreamSaver.js/mitm.html";

let writer: WritableStreamDefaultWriter<Uint8Array<ArrayBufferLike>> | null = null;
let isFirst = true;
const worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });

worker.addEventListener("message", async (event) => {
  if (event.data.type === "start") {
    const { filename } = event.data;
    const stream = streamSaver.createWriteStream(filename);
    writer = stream.getWriter();
  } else if (event.data.chunk) {
    if (isFirst) {
      isFirst = false;
      await writer!.write(new TextEncoder().encode(""));
    }

    const { id, data } = event.data.chunk;
    try {
      await writer!.write(data);
      worker.postMessage({ type: "chunkWritten", id });
    } catch (e) {
      worker.postMessage({ type: "chunkWritten", id, error: e?.toString() ?? String(e) });
    }
  } else if (event.data.type === "done") {
    await writer!.close();
    writer = null;
  }
});

// abort so it dose not look stuck
window.onunload = () => {
  writer?.abort();
};

const root = document.getElementById("root");
render(() => <App worker={worker} />, root!);
