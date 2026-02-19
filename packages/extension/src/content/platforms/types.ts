/** Interface that each platform adapter must implement */
export interface PlatformAdapter {
  /** Human-readable platform name */
  name: string;
  /** Returns true if the current page matches this platform */
  matches(): boolean;
  /** Get the chat input element */
  getChatInput(): HTMLElement | null;
  /** Inject text into the chat input */
  injectText(text: string): boolean;
  /** Get currently selected text on the page */
  getSelectedText(): string;
}

/**
 * Synthetic paste injection — works with paste-aware editors (Slate, Quill, etc.).
 * Creates a ClipboardEvent with a DataTransfer containing the text.
 */
export function syntheticPaste(el: HTMLElement, text: string): boolean {
  try {
    const dt = new DataTransfer();
    dt.setData("text/plain", text);
    const event = new ClipboardEvent("paste", {
      clipboardData: dt,
      bubbles: true,
      cancelable: true,
    });
    return el.dispatchEvent(event);
  } catch {
    return false;
  }
}
