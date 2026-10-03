/* @refresh reload */
import { render } from "solid-js/web";
import streamSaver from "streamsaver";
import App from "./App";
import "./index.css";

streamSaver.mitm = "/streamsaver/mitm.html";

let writer: WritableStreamDefaultWriter<Uint8Array<ArrayBufferLike>> | null = null;
const worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });

worker.addEventListener("message", async (event) => {
  if (event.data.type === "start") {
    const { filename } = event.data;
    const stream = streamSaver.createWriteStream(filename);
    writer = stream.getWriter();
  } else if (event.data.chunk) {
    const { id, data } = event.data.chunk;
    try {
      await writer!.write(data);
      worker.postMessage({ type: "chunkWritten", id });
    } catch (e) {
      console.log(e);
      worker.postMessage({ type: "chunkWritten", id, error: e?.toString() ?? String(e) });
    }
  } else if (event.data.type === "done") {
    await writer!.close();
    writer = null;
  }
});
console.log("PAGE IS LOADING");
// window.addEventListener("beforeunload", () => {
//   console.log("PAGE IS UNLOADING");
//   writer?.abort();
// });

const root = document.getElementById("root");
render(() => <App worker={worker} />, root!);
