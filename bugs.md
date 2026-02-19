# Bugs, Improvements & Roadmap

## Bugs

### API key creation fails in local mode
- **What:** Creating an API key throws "Failed to create API key" when using local vault (`npm run ui`).
- **Context:** Dashboard is accessible; only API key creation fails.

### Storage card number overflow
- **What:** Storage values display as long decimals (e.g. `1.85546875 MB`).
- **Fix:** Truncate to 1–2 decimal places.

### Search result click does nothing
- **What:** Search works, but clicking a result item has no effect.
- **Expected:** Should navigate or open the selected item.

### Drawer layout issues
- **What:** Drawer content lacks inner padding; metadata tab is a long list.
- **Fix:** Add padding; consider grid or more compact layout for metadata.

---

## UX Improvements

### Getting started card
- Check for valid MCP server config before showing completion state.
- General UX improvements.

### MCP server status
- Display somewhere: connected vs disconnected, local vs hosted.

---

## Feature Requests

### Data import flexibility
- Support formats beyond JSON.
- Allow uploading a folder or document of any kind.
- Auto-categorize and convert to vault structure (markdown + YAML).

---

## Roadmap

### External URL ingestion
- Accept URLs to hosted docs, blog articles, social posts, or video.
- Different pipelines per source type to convert to structured context documents.

### Pricing & free tier
- Review and refine pricing tiers and free user model over time.

### Account management
- Local users: sign in to / create cloud account.
- Connect local account to cloud account.
- Seamless transition between local and cloud.

### Export & data ownership
- **Principle:** Never lock data behind paywall; free users must be able to export.
- **Incentive for paid:** Managed hosting, storage, convenience.
- **Privacy:** Data always private, fully encrypted, accessible only to account owner.
- Improve export UX to avoid lock-in; easy data access is critical.
