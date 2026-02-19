import React from "react";
import type { SearchResult } from "@/shared/types";

interface Props {
  results: SearchResult[];
  query: string;
  onInject: (text: string) => void;
}

export function ResultList({ results, query, onInject }: Props) {
  if (!query) {
    return (
      <div style={{ padding: "24px 16px", textAlign: "center", color: "#64748b", fontSize: "13px" }}>
        Search your vault to find relevant context.
      </div>
    );
  }

  if (results.length === 0) {
    return (
      <div style={{ padding: "24px 16px", textAlign: "center", color: "#64748b", fontSize: "13px" }}>
        No results for "{query}"
      </div>
    );
  }

  return (
    <div style={{ padding: "0 16px 16px" }}>
      <div style={{ fontSize: "12px", color: "#64748b", marginBottom: "8px" }}>
        {results.length} result{results.length !== 1 ? "s" : ""}
      </div>
      {results.map((result) => (
        <ResultCard key={result.id} result={result} onInject={onInject} />
      ))}
    </div>
  );
}

function ResultCard({ result, onInject }: { result: SearchResult; onInject: (text: string) => void }) {
  const preview = result.body?.slice(0, 150) || "";
  const title = result.title || result.kind;

  function handleInject() {
    const text = result.body || "";
    onInject(text);
  }

  return (
    <div style={{
      padding: "10px 12px", marginBottom: "8px",
      backgroundColor: "#1e293b", borderRadius: "8px",
      border: "1px solid #334155",
    }}>
      <div style={{ display: "flex", justifyContent: "space-between", alignItems: "flex-start", marginBottom: "4px" }}>
        <div style={{ fontSize: "14px", fontWeight: 500, color: "#f1f5f9", flex: 1 }}>
          {title}
        </div>
        <span style={{
          fontSize: "11px", padding: "2px 6px",
          backgroundColor: "#334155", borderRadius: "4px", color: "#94a3b8",
          marginLeft: "8px", whiteSpace: "nowrap",
        }}>
          {result.kind}
        </span>
      </div>

      <div style={{ fontSize: "12px", color: "#94a3b8", marginBottom: "4px" }}>
        {result.score.toFixed(3)} · {result.tags.join(", ") || "no tags"}
      </div>

      <div style={{ fontSize: "13px", color: "#cbd5e1", lineHeight: "1.4", marginBottom: "8px" }}>
        {preview}{preview.length >= 150 ? "..." : ""}
      </div>

      <button
        onClick={handleInject}
        style={{
          fontSize: "12px", padding: "4px 10px",
          backgroundColor: "#3b82f6", color: "#fff",
          border: "none", borderRadius: "4px", cursor: "pointer",
        }}
      >
        Inject into chat
      </button>
    </div>
  );
}
