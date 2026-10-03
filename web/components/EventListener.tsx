import { onCleanup, onMount } from "solid-js";

export function EventListener<K extends keyof DocumentEventMap>(props: { event: K; handler(event: DocumentEventMap[K]): void }) {
  const { event, handler } = props;
  onMount(() => {
    document.addEventListener(event, handler);
    onCleanup(() => document.removeEventListener(event, handler));
  });
  return null;
}
