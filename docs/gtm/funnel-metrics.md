# Funnel Metrics and Gates

## Primary Funnel

1. Visit landing page
2. Register
3. Copy API key
4. First MCP call (`context_status`)
5. First `save_context`
6. First `get_context`
7. Upgrade click
8. Paid conversion

## 90-Day Targets

- 5k monthly sessions
- 20% visitor to register
- 35% register to activated
- 8-12% free to paid
- 12+ paid Pro users

## Launch Gates

- No production deploy if staging smoke fails
- No launch campaign if `/`, `/privacy`, `/api/vault/openapi.json` are not green
- No paid campaign before funnel instrumentation is live
