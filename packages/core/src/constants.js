/**
 * constants.js — Shared validation constants for vault entries.
 *
 * Canonical source for input size limits used across local (core tools) and
 * hosted (REST API validation). Import from here; never redefine locally.
 */

export const MAX_BODY_LENGTH = 100 * 1024; // 100KB
export const MAX_TITLE_LENGTH = 500;
export const MAX_KIND_LENGTH = 64;
export const MAX_TAG_LENGTH = 100;
export const MAX_TAGS_COUNT = 20;
export const MAX_META_LENGTH = 10 * 1024; // 10KB
export const MAX_SOURCE_LENGTH = 200;
export const MAX_IDENTITY_KEY_LENGTH = 200;
