-- Prompt hints shown per day, and how many of them a read returned within 30 minutes (OPS-473).
-- Run: sqlite3 ~/.context-mcp/vault.db < docs/hint-opens.sql
-- "Opened" means get_context returned the hinted entry within 30 minutes after the hook showed it.
-- The MCP server does not know the Claude session, so the window stands in for "the same session";
-- a read by another session in that window counts too, which can overstate the share a little.
-- Times are UTC ISO strings (hint_log.shown_at and the access_log rows written since OPS-473).
SELECT substr(h.shown_at, 1, 10) AS day,
       count(*) AS hints_shown,
       sum(EXISTS (SELECT 1 FROM access_log a
                   WHERE a.entry_id = h.entry_id
                     AND a.accessed_at >= h.shown_at
                     AND a.accessed_at <= strftime('%Y-%m-%dT%H:%M:%fZ', h.shown_at, '+30 minutes'))) AS opened,
       round(100.0 * sum(EXISTS (SELECT 1 FROM access_log a
                   WHERE a.entry_id = h.entry_id
                     AND a.accessed_at >= h.shown_at
                     AND a.accessed_at <= strftime('%Y-%m-%dT%H:%M:%fZ', h.shown_at, '+30 minutes'))) / count(*), 1) AS opened_pct
FROM hint_log h
GROUP BY day
ORDER BY day DESC;
