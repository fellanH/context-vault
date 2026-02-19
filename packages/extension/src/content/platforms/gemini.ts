import { syntheticPaste } from "./types";
import type { PlatformAdapter } from "./types";

export const geminiAdapter: PlatformAdapter = {
  name: "Gemini",

  matches() {
    return location.hostname === "gemini.google.com";
  },

  getChatInput() {
    // Fallback selector chain for Gemini's Quill-based editor
    return (
      document.querySelector<HTMLElement>('.ql-editor[contenteditable="true"]') ||
      document.querySelector<HTMLElement>('div[contenteditable="true"][role="textbox"]') ||
      document.querySelector<HTMLElement>('[contenteditable="true"]')
    );
  },

  injectText(text: string) {
    const input = this.getChatInput();
    if (!input) return false;

    input.focus();

    // Step 1: execCommand — works with Quill/rich text editors
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
