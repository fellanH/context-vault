export type BlogPostSection = {
  heading: string;
  paragraphs: string[];
};

export type BlogPost = {
  slug: string;
  title: string;
  description: string;
  category: "Integration" | "Playbook" | "Architecture";
  publishedAt: string;
  readTimeMinutes: number;
  ctaLabel: string;
  ctaHref: string;
  sections: BlogPostSection[];
};

export const posts: BlogPost[] = [
  {
    slug: "context-vault-claude-code-5-minute-setup",
    title: "Context Vault + Claude Code: 5-Minute Setup",
    description:
      "Install Context Vault, connect Claude Code over MCP, and verify your first persistent memory workflow.",
    category: "Integration",
    publishedAt: "2026-02-19",
    readTimeMinutes: 6,
    ctaLabel: "Start free",
    ctaHref: "/register",
    sections: [
      {
        heading: "Why this workflow matters",
        paragraphs: [
          "Most coding sessions restart context from scratch. Persistent memory removes repeated prompting and makes follow-up tasks faster.",
          "Context Vault gives Claude Code a reliable MCP memory layer so prior decisions, notes, and patterns are available across sessions.",
        ],
      },
      {
        heading: "Setup flow",
        paragraphs: [
          "Install the CLI globally and run setup. This configures your local vault, downloads embeddings, and validates tool health.",
          "Then connect your client with one MCP endpoint and verify your first tool call using context_status, followed by save_context and get_context.",
        ],
      },
      {
        heading: "Production checklist",
        paragraphs: [
          "Use one canonical MCP endpoint, keep your vault folder under version control where appropriate, and monitor first-run activation events.",
          "The highest-leverage metric is register to first successful get_context in under three minutes.",
        ],
      },
    ],
  },
  {
    slug: "moving-from-local-vault-to-hosted-without-lock-in",
    title: "Move From Local Vault To Hosted Without Lock-In",
    description:
      "A practical migration pattern to keep markdown portability while enabling managed hosted access.",
    category: "Playbook",
    publishedAt: "2026-02-18",
    readTimeMinutes: 7,
    ctaLabel: "See 2-minute setup",
    ctaHref:
      "https://github.com/fellanH/context-mcp/blob/main/docs/distribution/connect-in-2-minutes.md",
    sections: [
      {
        heading: "Keep your source of truth portable",
        paragraphs: [
          "Context Vault stores knowledge in markdown with YAML frontmatter. That gives you human-readable files and straightforward export behavior.",
          "Hosted usage adds convenience and distribution without forcing a proprietary store format.",
        ],
      },
      {
        heading: "Migration pattern",
        paragraphs: [
          "Keep the same information architecture (kind, tags, folder conventions) while introducing hosted auth and API key management.",
          "Validate retrieval quality after migration by sampling representative get_context queries and comparing top results.",
        ],
      },
      {
        heading: "What to measure",
        paragraphs: [
          "Track adoption by API key copy, first MCP call, first write, and first successful retrieval. These are stronger indicators than pageview metrics.",
          "If retrieval quality drops, tune kind granularity and recency behavior before expanding content volume.",
        ],
      },
    ],
  },
  {
    slug: "hybrid-search-for-agent-memory-quality",
    title: "Hybrid Search Is The Core Of Agent Memory Quality",
    description:
      "Why full-text + semantic retrieval with recency weighting matters when your memory corpus grows.",
    category: "Architecture",
    publishedAt: "2026-02-17",
    readTimeMinutes: 8,
    ctaLabel: "Start free",
    ctaHref: "/register",
    sections: [
      {
        heading: "Storage is not the hard part",
        paragraphs: [
          "The hard problem is returning the right five entries from thousands. Irrelevant retrieval wastes context window budget and hurts trust.",
          "Hybrid ranking balances exact keyword matching with semantic similarity so both explicit terms and intent are captured.",
        ],
      },
      {
        heading: "Recency and relevance",
        paragraphs: [
          "Session notes and architectural decisions age differently. A strong retrieval system accounts for these data lifecycles.",
          "Recency weighting should support freshness without burying durable decisions and patterns.",
        ],
      },
      {
        heading: "Operational guidance",
        paragraphs: [
          "Treat retrieval metrics as product metrics. Evaluate first-result usefulness, not only latency.",
          "Use periodic relevance checks to keep quality stable as your vault scales from hundreds to thousands of entries.",
        ],
      },
    ],
  },
];

export function getPostBySlug(slug?: string) {
  return posts.find((post) => post.slug === slug);
}
