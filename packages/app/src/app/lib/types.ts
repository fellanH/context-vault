export type Category = "knowledge" | "entity" | "event";
export type KnowledgeKind = "insight" | "decision" | "pattern";
export type EntityKind = "project" | "contact" | "tool";
export type EventKind = "session" | "log";

export type BillingTier = "free" | "pro" | "team";

// ─── Frontend types (used by components) ─────────────────────────────────────

export interface Entry {
  id: string;
  category: Category;
  kind: KnowledgeKind | EntityKind | EventKind;
  title: string;
  body: string;
  tags: string[];
  source?: string;
  created: Date;
  updated: Date;
  metadata?: Record<string, unknown>;
}

export interface SearchResult extends Entry {
  score: number;
}

export interface User {
  id: string;
  email: string;
  name?: string;
  tier: BillingTier;
  createdAt: Date;
}

export interface ApiKey {
  id: string;
  name: string;
  prefix: string;
  createdAt: Date;
}

export interface UsageResponse {
  entries: { used: number; limit: number };
  storage: { usedMb: number; limitMb: number };
  requestsToday: { used: number; limit: number };
  apiKeys: { active: number; limit: number };
}

export interface OnboardingStep {
  id: string;
  label: string;
  completed: boolean;
  description?: string;
}

// ─── API response types (match backend shapes exactly) ───────────────────────

export interface ApiEntry {
  id: string;
  kind: string;
  category: string;
  title: string | null;
  body: string | null;
  tags: string[];
  meta: Record<string, unknown>;
  source: string | null;
  identity_key: string | null;
  expires_at: string | null;
  created_at: string;
}

export interface ApiSearchResult extends ApiEntry {
  score: number;
}

export interface ApiKeyListItem {
  id: string;
  user_id: string;
  key_prefix: string;
  name: string;
  created_at: string;
}

export interface ApiUsageResponse {
  tier: BillingTier;
  limits: {
    maxEntries: number | "unlimited";
    requestsPerDay: number | "unlimited";
    storageMb: number;
    exportEnabled: boolean;
  };
  usage: {
    requestsToday: number;
    entriesUsed: number;
    storageMb: number;
  };
}

export interface ApiRegisterResponse {
  userId: string;
  email: string;
  tier: BillingTier;
  apiKey: {
    id: string;
    key: string;
    prefix: string;
    message: string;
  };
}

export interface ApiUserResponse {
  userId: string;
  email: string;
  name: string | null;
  tier: BillingTier;
  createdAt: string;
}

// ─── Transformers ────────────────────────────────────────────────────────────

export function transformEntry(raw: ApiEntry): Entry {
  return {
    id: raw.id,
    category: raw.category as Category,
    kind: raw.kind as Entry["kind"],
    title: raw.title || "",
    body: raw.body || "",
    tags: raw.tags || [],
    source: raw.source || undefined,
    created: new Date(raw.created_at),
    updated: new Date(raw.created_at), // backend doesn't track updated separately
    metadata: raw.meta && Object.keys(raw.meta).length > 0 ? raw.meta : undefined,
  };
}

export function transformSearchResult(raw: ApiSearchResult): SearchResult {
  return {
    ...transformEntry(raw),
    score: raw.score,
  };
}

export function transformApiKey(raw: ApiKeyListItem): ApiKey {
  return {
    id: raw.id,
    name: raw.name,
    prefix: raw.key_prefix,
    createdAt: new Date(raw.created_at),
  };
}

export function transformUsage(
  raw: ApiUsageResponse,
  apiKeyCount: number
): UsageResponse {
  const numOrMax = (v: number | "unlimited") =>
    v === "unlimited" ? Infinity : v;

  return {
    entries: {
      used: raw.usage.entriesUsed,
      limit: numOrMax(raw.limits.maxEntries),
    },
    storage: {
      usedMb: raw.usage.storageMb,
      limitMb: raw.limits.storageMb,
    },
    requestsToday: {
      used: raw.usage.requestsToday,
      limit: numOrMax(raw.limits.requestsPerDay),
    },
    apiKeys: {
      active: apiKeyCount,
      limit: Infinity, // not tracked in usage response
    },
  };
}

export function transformUser(raw: ApiUserResponse): User {
  return {
    id: raw.userId,
    email: raw.email,
    name: raw.name || undefined,
    tier: raw.tier,
    createdAt: new Date(raw.createdAt),
  };
}
