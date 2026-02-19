import { syntheticPaste } from "./types";
import type { PlatformAdapter } from "./types";

export const claudeAdapter: PlatformAdapter = {
  name: "Claude",

  matches() {
    return location.hostname === "claude.ai";
  },

  getChatInput() {
    // Fallback selector chain for Claude's ProseMirror editor
    return (
      document.querySelector<HTMLElement>('[contenteditable="true"].ProseMirror') ||
      document.querySelector<HTMLElement>("div.ProseMirror[contenteditable]") ||
      document.querySelector<HTMLElement>('[contenteditable="true"]')
    );
  },

  injectText(text: string) {
    const input = this.getChatInput();
    if (!input) return false;

    input.focus();

    // Step 1: execCommand — works with ProseMirror
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
