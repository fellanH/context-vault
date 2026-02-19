/** Vault entry as returned by the REST API */
export interface Entry {
  id: string;
  kind: string;
  category: "knowledge" | "entity" | "event";
  title: string | null;
  body: string | null;
  tags: string[];
  meta: Record<string, unknown>;
  source: string | null;
  identity_key: string | null;
  expires_at: string | null;
  created_at: string;
}

export interface SearchResult extends Entry {
  score: number;
}

/** Messages between popup/content scripts and background service worker */
export type MessageType =
  | { type: "search"; query: string; limit?: number }
  | { type: "search_result"; results: SearchResult[]; query: string }
  | { type: "inject_text"; text: string }
  | { type: "inject_result"; success: boolean }
  | { type: "capture"; kind: string; body: string; title?: string; tags?: string[] }
  | { type: "capture_result"; id: string }
  | { type: "get_settings" }
  | { type: "settings"; serverUrl: string; apiKey: string; connected: boolean }
  | { type: "save_settings"; serverUrl: string; apiKey: string }
  | { type: "test_connection" }
  | { type: "connection_result"; success: boolean; error?: string }
  | { type: "error"; message: string };

/** Extension storage shape */
export interface ExtensionSettings {
  serverUrl: string;
  apiKey: string;
}

/** Default settings */
export const DEFAULT_SETTINGS: ExtensionSettings = {
  serverUrl: "https://www.context-vault.com",
  apiKey: "",
};
