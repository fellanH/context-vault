import { syntheticPaste } from "./types";
import type { PlatformAdapter } from "./types";

export const chatgptAdapter: PlatformAdapter = {
  name: "ChatGPT",

  matches() {
    return location.hostname === "chatgpt.com" || location.hostname === "chat.openai.com";
  },

  getChatInput() {
    // Fallback selector chain for ChatGPT's prompt textarea
    return (
      document.querySelector<HTMLElement>("#prompt-textarea") ||
      document.querySelector<HTMLElement>('[data-testid="prompt-textarea"]') ||
      document.querySelector<HTMLElement>('div[contenteditable][role="textbox"]') ||
      document.querySelector<HTMLElement>('[contenteditable="true"]')
    );
  },

  injectText(text: string) {
    const input = this.getChatInput();
    if (!input) return false;

    input.focus();

    // Step 1: execCommand — works with ProseMirror/React synthetic events
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
