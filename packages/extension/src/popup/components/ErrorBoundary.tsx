import React from "react";

interface State { hasError: boolean; error: string | null }

export class ErrorBoundary extends React.Component<{ children: React.ReactNode }, State> {
  state: State = { hasError: false, error: null };

  static getDerivedStateFromError(error: Error): State {
    return { hasError: true, error: error.message };
  }

  render() {
    if (this.state.hasError) {
      return (
        <div style={{ padding: "24px", textAlign: "center" }}>
          <div style={{ fontSize: "24px", marginBottom: "12px" }}>Something went wrong</div>
          <div style={{ fontSize: "13px", color: "#94a3b8", marginBottom: "16px" }}>{this.state.error}</div>
          <button
            onClick={() => this.setState({ hasError: false, error: null })}
            style={{
              padding: "8px 16px", fontSize: "14px", backgroundColor: "#3b82f6",
              color: "#fff", border: "none", borderRadius: "6px", cursor: "pointer",
            }}
          >
            Try again
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}
