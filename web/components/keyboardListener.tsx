import { onCleanup, onMount } from "solid-js";

export interface KeyboardListenerProps {
  onKeyDown?(e: KeyboardEvent): void;
  onKeyUp?(e: KeyboardEvent): void;
  onKeyPress?(e: KeyboardEvent): void;

  /** If `true`, the listener will always be active, even if the user is typing / performing other actions. Defaults to `false`. */
  alwaysListen?: boolean;

  /** A list of keys to listen for. If not provided, all keys will be listened for. */
  keys?: string[];
}
export function KeyboardListener(props: KeyboardListenerProps) {
  const { keys = null, alwaysListen = false } = props;

  function handleKeyDown(e: KeyboardEvent) {
    if (!alwaysListen && isTyping()) return;
    if (!keys?.includes(e.key)) return;

    props.onKeyDown?.(e);
  }
  function handleKeyUp(e: KeyboardEvent) {
    if (!alwaysListen && isTyping()) return;
    if (!keys?.includes(e.key)) return;

    props.onKeyUp?.(e);
  }
  function handleKeyPress(e: KeyboardEvent) {
    if (!alwaysListen && isTyping()) return;
    if (!keys?.includes(e.key)) return;

    props.onKeyPress?.(e);
  }

  onMount(() => {
    document.addEventListener("keydown", handleKeyDown);
    document.addEventListener("keyup", handleKeyUp);
    document.addEventListener("keypress", handleKeyPress);

    onCleanup(() => {
      document.removeEventListener("keydown", handleKeyDown);
      document.removeEventListener("keyup", handleKeyUp);
      document.removeEventListener("keypress", handleKeyPress);
    });
  });

  return null;
}

/**
 * Checks if the user is currently typing in an input, textarea, or contenteditable element.
 *
 * Useful for enabling key commands when the active element shouldn't override key commands.
 */
export function isTyping() {
  return document.activeElement?.matches("input, textarea, select, [contenteditable]");
}
