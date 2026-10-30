export interface CopyButtonProps {
  text: string | (() => string | Promise<string>);

  onError?(error: unknown): void;
  onResult?(success: boolean, error?: unknown): void;
  onSuccess?(): void;
}
export default function CopyButton(props: CopyButtonProps) {
  return (
    <button
      class="button bg-gray-100 px-4"
      type="button"
      onClick={async (evt) => {
        evt.preventDefault();
        try {
          await navigator.clipboard.writeText(typeof props.text === "function" ? await props.text() : props.text);
          props.onSuccess ? props.onSuccess() : props.onResult?.(true);
        } catch (e: unknown) {
          props.onError ? props.onError(e) : props.onResult?.(false, e);
        }
      }}
    >
      Copy
    </button>
  );
}
