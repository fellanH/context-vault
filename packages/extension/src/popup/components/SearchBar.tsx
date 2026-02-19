import React, { useState, useRef, useEffect } from "react";

interface Props {
  onSearch: (query: string) => void;
  loading: boolean;
}

export function SearchBar({ onSearch, loading }: Props) {
  const [value, setValue] = useState("");
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    inputRef.current?.focus();
  }, []);

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    onSearch(value);
  }

  return (
    <form onSubmit={handleSubmit} style={{ padding: "12px 16px" }}>
      <div style={{ display: "flex", gap: "8px" }}>
        <input
          ref={inputRef}
          type="text"
          value={value}
          onChange={(e) => setValue(e.target.value)}
          placeholder="Search your vault..."
          style={{
            flex: 1, padding: "8px 12px", fontSize: "14px",
            backgroundColor: "#1e293b", border: "1px solid #334155",
            borderRadius: "6px", color: "#e2e8f0", outline: "none",
          }}
        />
        <button
          type="submit"
          disabled={loading || !value.trim()}
          style={{
            padding: "8px 16px", fontSize: "14px", fontWeight: 500,
            backgroundColor: loading ? "#334155" : "#3b82f6",
            color: "#fff", border: "none", borderRadius: "6px",
            cursor: loading ? "wait" : "pointer",
          }}
        >
          {loading ? "..." : "Search"}
        </button>
      </div>
    </form>
  );
}
