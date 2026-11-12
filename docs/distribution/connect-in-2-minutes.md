# Connect in 2 Minutes (Hosted)

Use Context Vault in the cloud: no local install, and it works across devices. Your AI tool connects to the hosted MCP endpoint and asks you to sign in once.

| Service   | URL                                    |
| --------- | -------------------------------------- |
| MCP       | `https://api.context-vault.com/mcp`    |
| Dashboard | `https://app.context-vault.com/`       |
| Health    | `https://api.context-vault.com/health` |
| Privacy   | `https://context-vault.com/privacy`    |

The hosted endpoint uses OAuth. API keys from the dashboard do not authorize `/mcp`; your client opens a sign-in page instead (Google or email), and you choose whether it may write.

---

## 1. Claude Code

```bash
claude mcp add --transport http context-vault https://api.context-vault.com/mcp
```

Then run `/mcp` in Claude Code, pick `context-vault` and sign in.

---

## 2. Cursor and other MCP clients

Add a remote MCP server with the URL `https://api.context-vault.com/mcp` and no headers. The client opens the sign-in page on first use.

---

## 3. Validate

In your AI client, run:

1. `context_status`, which shows your vault
2. `save_context` with a short insight
3. `get_context` with a matching query

For the free local vault instead, see [Get started](https://context-vault.com/get-started).
