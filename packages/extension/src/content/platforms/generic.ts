import { syntheticPaste } from "./types";
import type { PlatformAdapter } from "./types";

export const genericAdapter: PlatformAdapter = {
  name: "Generic",

  matches() {
    return true; // Fallback — always matches
  },

  getChatInput() {
    // Try contenteditable first, then textarea
    return (
      document.querySelector<HTMLElement>('[contenteditable="true"]') ||
      document.querySelector<HTMLElement>("textarea:not([readonly])") ||
      document.querySelector<HTMLElement>("input[type='text']:not([readonly])")
    );
  },

  injectText(text: string) {
    const input = this.getChatInput();
    if (!input) return false;

    input.focus();

    // For textarea/input elements — direct value manipulation
    if (input instanceof HTMLTextAreaElement || input instanceof HTMLInputElement) {
      const start = input.selectionStart ?? input.value.length;
      const end = input.selectionEnd ?? input.value.length;
      input.value = input.value.slice(0, start) + text + input.value.slice(end);
      input.selectionStart = input.selectionEnd = start + text.length;
      input.dispatchEvent(new Event("input", { bubbles: true }));
      return true;
    }

    // For contenteditable — 3-step injection chain
    // Step 1: execCommand — works with ProseMirror/Slate
    if (document.execCommand("insertText", false, text)) {
      return true;
    }

    // Step 2: Synthetic paste event — works with paste-aware editors
    if (syntheticPaste(input, text)) {
      return true;
    }

    // Step 3: Direct textContent set + input event — last resort
    input.textContent = (input.textContent || "") + text;
    input.dispatchEvent(new Event("input", { bubbles: true }));
    return true;
  },

  getSelectedText() {
    return window.getSelection()?.toString() || "";
  },
};
