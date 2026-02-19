# Connect in 2 Minutes

Canonical hosted base URL used in this repo:

- App: `https://www.context-vault.com/`
- MCP: `https://www.context-vault.com/mcp`
- OpenAPI: `https://www.context-vault.com/api/vault/openapi.json`
- Privacy: `https://www.context-vault.com/privacy`

## 1. Claude Code

Add this to your Claude Code MCP config:

```json
{
  "mcpServers": {
    "context-vault": {
      "url": "https://www.context-vault.com/mcp",
      "headers": {
        "Authorization": "Bearer YOUR_API_KEY"
      }
    }
  }
}
```

## 2. Cursor

Configure an MCP server in Cursor with the same endpoint and bearer token:

- URL: `https://www.context-vault.com/mcp`
- Header: `Authorization: Bearer YOUR_API_KEY`

## 3. GPT Actions

In GPT Actions:

1. Import OpenAPI from `https://www.context-vault.com/api/vault/openapi.json`
2. Configure Bearer auth with your `cv_...` key
3. Set privacy URL to `https://www.context-vault.com/privacy`

## 4. Validate

After setup, run this sequence in your client:

1. `context_status`
2. `save_context` with a short insight
3. `get_context` with a matching query

If all three succeed, the integration is production-ready.
