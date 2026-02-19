(function () {
  'use strict';

  function syntheticPaste(el, text) {
    try {
      const dt = new DataTransfer();
      dt.setData("text/plain", text);
      const event = new ClipboardEvent("paste", {
        clipboardData: dt,
        bubbles: true,
        cancelable: true
      });
      return el.dispatchEvent(event);
    } catch {
      return false;
    }
  }

  const chatgptAdapter = {
    name: "ChatGPT",
    matches() {
      return location.hostname === "chatgpt.com" || location.hostname === "chat.openai.com";
    },
    getChatInput() {
      return document.querySelector("#prompt-textarea") || document.querySelector('[data-testid="prompt-textarea"]') || document.querySelector('div[contenteditable][role="textbox"]') || document.querySelector('[contenteditable="true"]');
    },
    injectText(text) {
      const input = this.getChatInput();
      if (!input) return false;
      input.focus();
      if (document.execCommand("insertText", false, text)) {
        return true;
      }
      if (syntheticPaste(input, text)) {
        return true;
      }
      input.textContent = (input.textContent || "") + text;
      input.dispatchEvent(new Event("input", { bubbles: true }));
      return true;
    },
    getSelectedText() {
      return window.getSelection()?.toString() || "";
    }
  };

  const claudeAdapter = {
    name: "Claude",
    matches() {
      return location.hostname === "claude.ai";
    },
    getChatInput() {
      return document.querySelector('[contenteditable="true"].ProseMirror') || document.querySelector("div.ProseMirror[contenteditable]") || document.querySelector('[contenteditable="true"]');
    },
    injectText(text) {
      const input = this.getChatInput();
      if (!input) return false;
      input.focus();
      if (document.execCommand("insertText", false, text)) {
        return true;
      }
      if (syntheticPaste(input, text)) {
        return true;
      }
      input.textContent = (input.textContent || "") + text;
      input.dispatchEvent(new Event("input", { bubbles: true }));
      return true;
    },
    getSelectedText() {
      return window.getSelection()?.toString() || "";
    }
  };

  const geminiAdapter = {
    name: "Gemini",
    matches() {
      return location.hostname === "gemini.google.com";
    },
    getChatInput() {
      return document.querySelector('.ql-editor[contenteditable="true"]') || document.querySelector('div[contenteditable="true"][role="textbox"]') || document.querySelector('[contenteditable="true"]');
    },
    injectText(text) {
      const input = this.getChatInput();
      if (!input) return false;
      input.focus();
      if (document.execCommand("insertText", false, text)) {
        return true;
      }
      if (syntheticPaste(input, text)) {
        return true;
      }
      input.textContent = (input.textContent || "") + text;
      input.dispatchEvent(new Event("input", { bubbles: true }));
      return true;
    },
    getSelectedText() {
      return window.getSelection()?.toString() || "";
    }
  };

  const genericAdapter = {
    name: "Generic",
    matches() {
      return true;
    },
    getChatInput() {
      return document.querySelector('[contenteditable="true"]') || document.querySelector("textarea:not([readonly])") || document.querySelector("input[type='text']:not([readonly])");
    },
    injectText(text) {
      const input = this.getChatInput();
      if (!input) return false;
      input.focus();
      if (input instanceof HTMLTextAreaElement || input instanceof HTMLInputElement) {
        const start = input.selectionStart ?? input.value.length;
        const end = input.selectionEnd ?? input.value.length;
        input.value = input.value.slice(0, start) + text + input.value.slice(end);
        input.selectionStart = input.selectionEnd = start + text.length;
        input.dispatchEvent(new Event("input", { bubbles: true }));
        return true;
      }
      if (document.execCommand("insertText", false, text)) {
        return true;
      }
      if (syntheticPaste(input, text)) {
        return true;
      }
      input.textContent = (input.textContent || "") + text;
      input.dispatchEvent(new Event("input", { bubbles: true }));
      return true;
    },
    getSelectedText() {
      return window.getSelection()?.toString() || "";
    }
  };

  const adapters = [chatgptAdapter, claudeAdapter, geminiAdapter, genericAdapter];
  function detectPlatform() {
    for (const adapter of adapters) {
      if (adapter.matches()) return adapter;
    }
    return genericAdapter;
  }

  const platform = detectPlatform();
  console.log(`[context-vault] Content script loaded on ${platform.name}`);
  chrome.runtime.onMessage.addListener(
    (message, _sender, sendResponse) => {
      switch (message.type) {
        case "inject_text": {
          const success = platform.injectText(message.text);
          sendResponse({ type: "inject_result", success });
          break;
        }
        case "capture_result": {
          showNotification(`Saved to vault (${message.id.slice(0, 8)}...)`, "success");
          break;
        }
        case "error": {
          showNotification(message.message, "error");
          break;
        }
      }
      return false;
    }
  );
  function showNotification(text, type) {
    const existing = document.getElementById("context-vault-host");
    if (existing) existing.remove();
    const host = document.createElement("div");
    host.id = "context-vault-host";
    Object.assign(host.style, {
      position: "fixed",
      bottom: "20px",
      right: "20px",
      zIndex: "999999"
    });
    const shadow = host.attachShadow({ mode: "closed" });
    const style = document.createElement("style");
    style.textContent = `
    .toast {
      padding: 12px 20px;
      border-radius: 8px;
      font-size: 14px;
      font-family: system-ui, -apple-system, sans-serif;
      color: #fff;
      background-color: ${type === "success" ? "#22c55e" : "#ef4444"};
      box-shadow: 0 4px 12px rgba(0, 0, 0, 0.15);
      transition: opacity 0.3s ease;
      opacity: 1;
    }
  `;
    const toast = document.createElement("div");
    toast.className = "toast";
    toast.textContent = text;
    shadow.appendChild(style);
    shadow.appendChild(toast);
    document.body.appendChild(host);
    setTimeout(() => {
      toast.style.opacity = "0";
      setTimeout(() => host.remove(), 300);
    }, 3e3);
  }

})();
