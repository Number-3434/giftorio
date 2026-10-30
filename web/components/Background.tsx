import gumpMp4 from "@/assets/img/gump.mp4";
import gumpWebm from "@/assets/img/gump.webm";
import nyanGif from "@/assets/img/nyan.gif";
import nyanMp4 from "@/assets/img/nyan.mp4";
import nyanWebm from "@/assets/img/nyan.webm";
import rickMp4 from "@/assets/img/rick.mp4";
import rickWebm from "@/assets/img/rick.webm";
import { createSignal, onCleanup, onMount } from "solid-js";

const MEDIA = [
  { mp4: nyanMp4, webm: nyanWebm, fallback: nyanGif },
  { mp4: rickMp4, webm: rickWebm, fallback: nyanGif },
  { mp4: gumpMp4, webm: gumpWebm, fallback: nyanGif },
] as const;

const TRANSITION_DURATION = 1000;

export interface BackgroundApi {
  setImageURL(url: string | null | undefined): void;
}

export interface BackgroundProps {
  ref?(api: BackgroundApi): void;
  interval?: number;
}

export default function Background(props: BackgroundProps) {
  const [activeSlot, setActiveSlot] = createSignal<number | null>(null);
  const [imageURL, setImageURL] = createSignal<string>();
  const [imageVisible, setImageVisible] = createSignal(false);

  let videos: HTMLVideoElement[] = [];
  let currentIndex = -1;
  let currentSlot = 0;
  let busy = false;
  let disposed = false;
  let interval: number | undefined;
  let transitionTimeout: number | undefined;
  let requestId = 0;

  function releaseVideo(video: HTMLVideoElement) {
    video.pause();
    video.removeAttribute("src");
    video.load();
  }

  function getVideoSource(video: HTMLVideoElement, media: (typeof MEDIA)[number]) {
    // Prefer MP4, falling back to WebM if the browser cannot play it.
    if (video.canPlayType("video/mp4")) return media.mp4;
    if (video.canPlayType("video/webm")) return media.webm;
    return null;
  }

  function loadVideo(video: HTMLVideoElement, index: number): Promise<void> {
    return new Promise((resolve, reject) => {
      const source = getVideoSource(video, MEDIA[index]);
      if (!source) {
        reject(new Error("No supported video format"));
        return;
      }

      function cleanup() {
        video.removeEventListener("loadeddata", onLoaded);
        video.removeEventListener("error", onError);
      }
      function onLoaded() {
        cleanup();
        resolve();
      }
      function onError() {
        cleanup();
        reject(new Error("Video failed to load"));
      }

      video.addEventListener("loadeddata", onLoaded, { once: true });
      video.addEventListener("error", onError, { once: true });

      video.src = source;
      video.load();
    });
  }

  function pickNextIndex(): number {
    if (MEDIA.length <= 1) return currentIndex;

    let index: number;
    do index = Math.floor(Math.random() * MEDIA.length);
    while (index === currentIndex);

    return index;
  }

  async function switchToVideo(index: number) {
    if (disposed || busy || imageURL() || index === currentIndex || index < 0) return;

    const request = ++requestId;
    const incomingSlot = currentSlot === 0 ? 1 : 0;
    const outgoingSlot = currentSlot;
    const incomingVideo = videos[incomingSlot];
    const outgoingVideo = videos[outgoingSlot];

    if (!incomingVideo || !outgoingVideo) return;

    busy = true;
    window.clearTimeout(transitionTimeout);

    releaseVideo(incomingVideo);

    try {
      await loadVideo(incomingVideo, index);
      if (disposed || request !== requestId) return;

      await incomingVideo.play();
      if (disposed || request !== requestId) return;

      setActiveSlot(incomingSlot);
      setImageVisible(false);

      currentIndex = index;
      currentSlot = incomingSlot;

      transitionTimeout = window.setTimeout(() => {
        if (disposed || request !== requestId) return;

        releaseVideo(outgoingVideo);
        busy = false;
      }, TRANSITION_DURATION);
    } catch {
      if (disposed || request !== requestId) return;

      releaseVideo(incomingVideo);
      setImageURL(MEDIA[index].fallback);
      currentIndex = index;
      setImageVisible(false);
      busy = false;
    }
  }

  function showCustomImage(url: string) {
    requestId++;
    busy = false;

    window.clearTimeout(transitionTimeout);

    setImageVisible(false);
    setActiveSlot(null);
    setImageURL(url);

    for (const video of videos) {
      releaseVideo(video);
    }
  }

  function clearCustomImage() {
    requestId++;
    setImageVisible(false);
    setImageURL(undefined);

    if (currentIndex >= 0) {
      void switchToVideo(pickNextIndex());
    }
  }

  props.ref?.({
    setImageURL(url) {
      url ? showCustomImage(url) : clearCustomImage();
    },
  } satisfies BackgroundApi);

  onMount(() => {
    const initialIndex = Math.floor(Math.random() * MEDIA.length);
    const initialVideo = videos[0];

    if (initialVideo) {
      busy = true;

      void loadVideo(initialVideo, initialIndex)
        .then(() => initialVideo.play())
        .then(() => {
          if (disposed) return;
          currentIndex = initialIndex;
          currentSlot = 0;
          setActiveSlot(0);
        })
        .catch(() => {
          if (disposed) return;
          currentIndex = initialIndex;
          setImageURL(MEDIA[initialIndex].fallback);
        })
        .finally(() => (busy = false));
    }

    interval = window.setInterval(() => {
      if (busy || imageURL()) return;
      void switchToVideo(pickNextIndex());
    }, props.interval ?? 10000);
  });

  onCleanup(() => {
    disposed = true;

    if (interval !== undefined) {
      window.clearInterval(interval);
    }
    window.clearTimeout(transitionTimeout);

    for (const video of videos) {
      releaseVideo(video);
    }
  });

  return (
    <div id="media-container" class="fixed top-0 left-0 w-full h-full z-[-1]">
      <div class="absolute inset-0 bg-black" />

      <video
        ref={(el) => (videos[0] = el)}
        class="absolute top-0 left-0 w-full h-full object-cover transition-opacity duration-1000 ease-in-out opacity-0 z-0"
        classList={{ "opacity-100": activeSlot() === 0 && !imageVisible() }}
        muted
        loop
        playsinline
        preload="none"
      />

      <video
        ref={(el) => (videos[1] = el)}
        class="absolute top-0 left-0 w-full h-full object-cover transition-opacity duration-1000 ease-in-out opacity-0 z-0"
        classList={{ "opacity-100": activeSlot() === 1 && !imageVisible() }}
        muted
        loop
        playsinline
        preload="none"
      />

      <img
        src={imageURL()}
        alt=""
        onLoad={() => imageURL() && setImageVisible(true)}
        class="absolute top-0 left-0 w-full h-full object-contain transition-opacity duration-1000 ease-in-out opacity-0 z-10"
        classList={{ "opacity-30": imageVisible() }}
        style={{ "image-rendering": "pixelated" }}
      />
    </div>
  );
}
