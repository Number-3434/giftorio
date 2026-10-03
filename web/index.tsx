const AUTOCLOSE_WINDOW_KEY = "giftorio_autoclose_window";
if (new URLSearchParams(location.search).get(AUTOCLOSE_WINDOW_KEY) === "true") {
  window.close();
}

import { render } from "solid-js/web";
import streamSaver from "streamsaver";
import App from "./App";
import "./index.css";

const mitm = new URL("/streamsaver/mitm.html", location.href);
const scope = new URL("./", mitm.href);
const org = location.origin.replace(/(^\w+:|^)\/\//, "");

streamSaver.mitm = "/streamsaver/mitm.html";

let downloadUrl: string | null = null;
let lastWritePromise: Promise<void> = Promise.resolve();
let writer: WritableStreamDefaultWriter<Uint8Array<ArrayBufferLike>> | null = null;
const worker = new Worker(new URL("./worker.js", import.meta.url), { type: "module" });

worker.addEventListener("message", async (event) => {
  if (event.data.type === "start") {
    console.log(streamSaver.supported);
    const { filename } = event.data;
    const pathname = [Math.random(), Date.now(), filename].join("/");

    downloadUrl = new URL(`${scope.href}${org}/${pathname}`).toString();
    writer = streamSaver.createWriteStream(filename, { pathname }).getWriter();
    worker.postMessage({ type: "writerReady" });
  } else if (event.data.chunk) {
    const { id, data } = event.data.chunk;

    try {
      lastWritePromise = writer!.write(data);
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

const openDownloadWindow = () => {
  const url = new URL(downloadUrl!);
  url.searchParams.set(AUTOCLOSE_WINDOW_KEY, "true");
  return downloadUrl && window.open(downloadUrl, "_blank", "width=100,height=100");
};

const root = document.getElementById("root");
render(() => <App worker={worker} openDownloadWindow={openDownloadWindow} />, root!);
