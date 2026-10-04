import { render } from "solid-js/web";
import streamSaver from "streamsaver"; // If unsupported, this will fall back to a polyfill that buffers in RAM
import App, { AppApi } from "./App";
import "./index.css";

let ref: AppApi;

const mitm = new URL("/streamsaver/mitm.html", location.href);
const scope = new URL("./", mitm.href);
const org = location.origin.replace(/(^\w+:|^)\/\//, "");

streamSaver.mitm = "/streamsaver/mitm.html";

let downloadUrl: string | null = null;
let writer: WritableStreamDefaultWriter<Uint8Array<ArrayBufferLike>> | null = null;
let prevWrites: Promise<void>[] = [];
let isUnblockingWriter = false;

const worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });

/**
 * On Firefox this opens a new tab with minimal size and immediately closes it.
 *
 * The tab makes a user-defined GET request to the download URL which unblocks the writer.
 * Note that making a regular GET request to the URL results in Vite redirecting to index.html
 */
async function unblockWriter() {
  // Test 10 writes of empty data. The writer internally buffers writes to reduce backpressure,
  // so we are unable to detect if it is blocked from testing a singular write. Typically this will
  // block on the 2nd or 3rd writes. If 250ms pass without completing the test we open a new tab.
  isUnblockingWriter = true;

  const writes = Array.from({ length: 10 }, () => writer!.write(new Uint8Array()));
  const timeout = new Promise((_, reject) => setTimeout(reject, 250));

  await Promise.race([Promise.all(writes), timeout])
    .catch(() => openDownloadWindow())
    .finally(() => (isUnblockingWriter = false));
}

worker.addEventListener("message", async (event) => {
  if (event.data.type === "start") {
    const { filename } = event.data;
    const pathname = [Math.random(), Date.now(), filename].join("/");

    downloadUrl = new URL(`${scope.href}${org}/${pathname}`).toString();
    writer = streamSaver.createWriteStream(filename, { pathname }).getWriter();

    unblockWriter(); // Workaround for Firefox

    worker.postMessage({ type: "writerReady" });
  } else if (event.data.chunk) {
    const { id, data } = event.data.chunk;
    try {
      if (isUnblockingWriter) {
        prevWrites.push(writer!.write(data));
      } else {
        await Promise.all(prevWrites);
        await writer!.write(data);
        prevWrites.length = 0;
      }
      worker.postMessage({ type: "chunkWritten", id });
    } catch (e) {
      console.log(e);
      worker.postMessage({ type: "chunkWritten", id, error: e?.toString() ?? String(e) });
    }
  } else if (event.data.type === "done") {
    try {
      await writer!.close();
    } finally {
      writer = null;
    }
  }
});

window.addEventListener("beforeunload", () => {
  writer?.abort();
});

function openDownloadWindow() {
  ref.showOpenStreamingWindowWarning();
  return window.open(downloadUrl!, "Giftorio: Streaming Download Unblocker", "noopener,noreferrer,left=0,top=0,width=100,height=100");
}
const root = document.getElementById("root");

render(() => <App ref={(api) => (ref = api)} worker={worker} streamingAvailable={streamSaver.supported} />, root!);
