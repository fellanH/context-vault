let cachedSettings = null;
async function getSettings() {
  if (cachedSettings) return cachedSettings;
  const result = await chrome.storage.local.get(["serverUrl", "apiKey"]);
  cachedSettings = {
    serverUrl: result.serverUrl || "",
    apiKey: result.apiKey || ""
  };
  return cachedSettings;
}
function clearSettingsCache() {
  cachedSettings = null;
}
const RETRY_BACKOFF_MS = [1e3, 3e3];
const FETCH_TIMEOUT_MS = 15e3;
const NO_RETRY_STATUSES = /* @__PURE__ */ new Set([401, 429]);
async function apiFetch(path, options = {}) {
  const { serverUrl, apiKey } = await getSettings();
  if (!serverUrl || !apiKey) throw new Error("Not configured — set server URL and API key in extension settings");
  const url = `${serverUrl.replace(/\/$/, "")}${path}`;
  const headers = {
    "Content-Type": "application/json",
    Authorization: `Bearer ${apiKey}`,
    ...options.headers || {}
  };
  let lastError;
  for (let attempt = 0; attempt <= RETRY_BACKOFF_MS.length; attempt++) {
    try {
      const controller = new AbortController();
      const timeout = setTimeout(() => controller.abort(), FETCH_TIMEOUT_MS);
      const res = await fetch(url, { ...options, headers, signal: controller.signal });
      clearTimeout(timeout);
      if (!res.ok) {
        const body = await res.json().catch(() => ({ error: res.statusText }));
        const err = new Error(body.error || `API error: ${res.status}`);
        err.status = res.status;
        if (NO_RETRY_STATUSES.has(res.status)) throw err;
        lastError = err;
        if (attempt < RETRY_BACKOFF_MS.length) {
          await new Promise((r) => setTimeout(r, RETRY_BACKOFF_MS[attempt]));
          continue;
        }
        throw err;
      }
      const remaining = res.headers.get("X-RateLimit-Remaining");
      const reset = res.headers.get("X-RateLimit-Reset");
      if (remaining !== null || reset !== null) {
        const rateLimitData = {};
        if (remaining !== null) rateLimitData.rateLimitRemaining = remaining;
        if (reset !== null) rateLimitData.rateLimitReset = reset;
        chrome.storage.local.set(rateLimitData);
      }
      return res.json();
    } catch (err) {
      if (err instanceof DOMException && err.name === "AbortError") {
        lastError = new Error(`Request timed out after ${FETCH_TIMEOUT_MS}ms`);
        if (attempt < RETRY_BACKOFF_MS.length) {
          await new Promise((r) => setTimeout(r, RETRY_BACKOFF_MS[attempt]));
          continue;
        }
      }
      if (err.status && NO_RETRY_STATUSES.has(err.status)) throw err;
      lastError = err instanceof Error ? err : new Error(String(err));
      if (attempt < RETRY_BACKOFF_MS.length) {
        await new Promise((r) => setTimeout(r, RETRY_BACKOFF_MS[attempt]));
        continue;
      }
    }
  }
  throw lastError || new Error("apiFetch failed after retries");
}
async function searchVault(query, opts = {}) {
  return apiFetch("/api/vault/search", {
    method: "POST",
    body: JSON.stringify({ query, ...opts })
  });
}
async function createEntry(data) {
  return apiFetch("/api/vault/entries", {
    method: "POST",
    body: JSON.stringify({ ...data, source: data.source || "browser-extension" })
  });
}
async function getVaultStatus() {
  return apiFetch("/api/vault/status");
}

const DEFAULT_SETTINGS = {
  serverUrl: "https://www.context-vault.com"};

const CONTEXT_MENU_PARENT_ID = "save-to-vault";
const CONTEXT_MENU_VARIANTS = [
  { id: "save-as-insight", title: "Save as Insight", kind: "insight", tags: ["captured", "insight"] },
  { id: "save-as-note", title: "Save as Note", kind: "note", tags: ["captured", "note"] },
  { id: "save-as-reference", title: "Save as Reference", kind: "reference", tags: ["captured", "reference"] },
  { id: "save-as-snippet", title: "Save as Code Snippet", kind: "snippet", tags: ["captured", "code"] }
];
function updateBadge(connected) {
  if (connected) {
    chrome.action.setBadgeText({ text: "" });
  } else {
    chrome.action.setBadgeText({ text: "!" });
    chrome.action.setBadgeBackgroundColor({ color: "#ef4444" });
  }
}
function originPatternFromServerUrl(serverUrl) {
  let parsed;
  try {
    parsed = new URL(serverUrl.trim());
  } catch {
    throw new Error("Invalid server URL. Use a full URL like https://www.context-vault.com");
  }
  if (parsed.protocol !== "https:" && parsed.protocol !== "http:") {
    throw new Error("Server URL must use http:// or https://");
  }
  return `${parsed.protocol}//${parsed.hostname}/*`;
}
function containsOriginPermission(origin) {
  return new Promise((resolve) => {
    chrome.permissions.contains({ origins: [origin] }, (granted) => resolve(Boolean(granted)));
  });
}
function requestOriginPermission(origin) {
  return new Promise((resolve) => {
    chrome.permissions.request({ origins: [origin] }, (granted) => resolve(Boolean(granted)));
  });
}
async function ensureServerPermission(serverUrl) {
  const origin = originPatternFromServerUrl(serverUrl);
  const hasPermission = await containsOriginPermission(origin);
  if (hasPermission) return origin;
  const granted = await requestOriginPermission(origin);
  if (!granted) {
    throw new Error(`Permission denied for ${origin}. Allow host access to connect this server.`);
  }
  return origin;
}
function setupContextMenus() {
  chrome.contextMenus.removeAll(() => {
    chrome.contextMenus.create({
      id: CONTEXT_MENU_PARENT_ID,
      title: "Save to Context Vault",
      contexts: ["selection"]
    });
    for (const item of CONTEXT_MENU_VARIANTS) {
      chrome.contextMenus.create({
        id: item.id,
        parentId: CONTEXT_MENU_PARENT_ID,
        title: item.title,
        contexts: ["selection"]
      });
    }
  });
}
chrome.runtime.onInstalled.addListener((details) => {
  setupContextMenus();
  chrome.storage.local.get(["apiKey"], (stored) => {
    updateBadge(Boolean(stored.apiKey));
  });
  if (details.reason === "install") {
    chrome.tabs.create({ url: chrome.runtime.getURL("onboarding/index.html") });
  }
});
chrome.contextMenus.onClicked.addListener(async (info, tab) => {
  const selected = info.selectionText?.trim();
  if (!selected) return;
  const variant = CONTEXT_MENU_VARIANTS.find((item) => item.id === info.menuItemId);
  if (!variant) return;
  try {
    const source = (() => {
      try {
        return tab?.url ? new URL(tab.url).hostname : "browser-extension";
      } catch {
        return "browser-extension";
      }
    })();
    const entry = await createEntry({
      kind: variant.kind,
      body: selected,
      title: selected.slice(0, 80) + (selected.length > 80 ? "..." : ""),
      source,
      tags: [...variant.tags]
    });
    if (tab?.id) {
      chrome.tabs.sendMessage(tab.id, {
        type: "capture_result",
        id: entry.id
      });
    }
  } catch (err) {
    console.error("[context-vault] Save failed:", err);
    if (tab?.id) {
      chrome.tabs.sendMessage(tab.id, {
        type: "error",
        message: err instanceof Error ? err.message : "Save failed"
      });
    }
  }
});
chrome.runtime.onMessage.addListener(
  (message, _sender, sendResponse) => {
    handleMessage(message).then(sendResponse).catch(
      (err) => sendResponse({ type: "error", message: err instanceof Error ? err.message : "Unknown error" })
    );
    return true;
  }
);
async function handleMessage(message) {
  switch (message.type) {
    case "search": {
      const result = await searchVault(message.query, { limit: message.limit || 10 });
      return { type: "search_result", results: result.results, query: result.query };
    }
    case "capture": {
      const entry = await createEntry({
        kind: message.kind,
        body: message.body,
        title: message.title,
        tags: message.tags
      });
      return { type: "capture_result", id: entry.id };
    }
    case "get_settings": {
      const stored = await chrome.storage.local.get(["serverUrl", "apiKey"]);
      const serverUrl = stored.serverUrl || DEFAULT_SETTINGS.serverUrl;
      const apiKey = stored.apiKey || "";
      return {
        type: "settings",
        serverUrl,
        apiKey,
        connected: Boolean(apiKey)
      };
    }
    case "save_settings": {
      const serverUrl = message.serverUrl.trim().replace(/\/$/, "");
      const apiKey = message.apiKey.trim();
      if (!serverUrl) {
        return { type: "error", message: "Server URL is required" };
      }
      try {
        await ensureServerPermission(serverUrl);
      } catch (err) {
        return { type: "error", message: err instanceof Error ? err.message : "Permission request failed" };
      }
      await chrome.storage.local.set({ serverUrl, apiKey });
      clearSettingsCache();
      updateBadge(Boolean(apiKey));
      return { type: "settings", serverUrl, apiKey, connected: Boolean(serverUrl && apiKey) };
    }
    case "test_connection": {
      try {
        const status = await getVaultStatus();
        const connected = status.health === "ok" || status.health === "degraded";
        updateBadge(connected);
        return { type: "connection_result", success: connected };
      } catch (err) {
        updateBadge(false);
        return { type: "connection_result", success: false, error: err instanceof Error ? err.message : "Connection failed" };
      }
    }
    default:
      return { type: "error", message: "Unknown message type" };
  }
}
