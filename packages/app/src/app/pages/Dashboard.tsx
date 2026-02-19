import { useState } from "react";
import { Link } from "react-router";
import { Card, CardContent, CardHeader, CardTitle } from "../components/ui/card";
import { Badge } from "../components/ui/badge";
import { Button } from "../components/ui/button";
import { Checkbox } from "../components/ui/checkbox";
import { UsageMeter } from "../components/UsageMeter";
import { useEntries, useUsage, useApiKeys } from "../lib/hooks";
import { useAuth } from "../lib/auth";
import {
  getOnboardingSteps,
  toggleOnboardingStep,
  isOnboardingDismissed,
  dismissOnboarding,
} from "../lib/onboarding";
import {
  FileText,
  HardDrive,
  Zap,
  Key,
  Search,
  Plus,
  Upload,
  X,
  Copy,
  Check,
  ExternalLink,
  Loader2,
} from "lucide-react";
import type { OnboardingStep } from "../lib/types";
import { toast } from "sonner";

function formatRelativeTime(date: Date): string {
  const now = new Date();
  const diffMs = now.getTime() - date.getTime();
  const diffMins = Math.floor(diffMs / 60000);
  const diffHours = Math.floor(diffMs / 3600000);
  const diffDays = Math.floor(diffMs / 86400000);

  if (diffMins < 60) return `${diffMins}m ago`;
  if (diffHours < 24) return `${diffHours}h ago`;
  return `${diffDays}d ago`;
}

export function Dashboard() {
  const { isAuthenticated } = useAuth();
  const { data: entriesData, isLoading: entriesLoading } = useEntries({ limit: 10 });
  const { data: usage, isLoading: usageLoading } = useUsage();
  const { data: apiKeys } = useApiKeys();

  const entriesUsed = usage?.entries.used ?? 0;
  const [steps, setSteps] = useState<OnboardingStep[]>(() =>
    getOnboardingSteps(isAuthenticated, entriesUsed)
  );
  const [showChecklist, setShowChecklist] = useState(() => !isOnboardingDismissed());
  const [copiedConfig, setCopiedConfig] = useState(false);

  // Re-derive steps when usage loads
  const derivedSteps = getOnboardingSteps(isAuthenticated, entriesUsed);
  if (derivedSteps.some((s, i) => s.completed !== steps[i]?.completed)) {
    setSteps(derivedSteps);
  }

  const allComplete = steps.every((s) => s.completed);

  const toggleStep = (id: string) => {
    const updated = toggleOnboardingStep(id);
    setSteps(updated);
  };

  const handleDismiss = () => {
    dismissOnboarding();
    setShowChecklist(false);
  };

  const copyMcpConfig = async () => {
    const config = JSON.stringify({
      mcpServers: {
        "context-vault": {
          url: "https://www.context-vault.com/mcp",
          headers: { Authorization: "Bearer YOUR_API_KEY" },
        },
      },
    }, null, 2);
    await navigator.clipboard.writeText(config);
    setCopiedConfig(true);
    toast.success("MCP config copied");
    setTimeout(() => setCopiedConfig(false), 2000);
  };

  const isUnlimited = (limit: number) => !isFinite(limit);

  const usageCards = usage
    ? [
        {
          label: "Entries",
          icon: FileText,
          used: usage.entries.used,
          limit: usage.entries.limit,
          display: `${usage.entries.used}`,
          sub: isUnlimited(usage.entries.limit) ? null : `of ${usage.entries.limit}`,
        },
        {
          label: "Storage",
          icon: HardDrive,
          used: usage.storage.usedMb,
          limit: usage.storage.limitMb,
          display: `${usage.storage.usedMb} MB`,
          sub: isUnlimited(usage.storage.limitMb) ? null : `of ${usage.storage.limitMb} MB`,
        },
        {
          label: "Requests Today",
          icon: Zap,
          used: usage.requestsToday.used,
          limit: usage.requestsToday.limit,
          display: `${usage.requestsToday.used}`,
          sub: isUnlimited(usage.requestsToday.limit) ? null : `of ${usage.requestsToday.limit}`,
        },
        {
          label: "API Keys",
          icon: Key,
          used: apiKeys?.length ?? 0,
          limit: Infinity,
          display: `${apiKeys?.length ?? 0} active`,
          sub: null,
        },
      ]
    : [];

  const entries = entriesData?.entries ?? [];

  return (
    <div className="p-6 space-y-6 max-w-5xl mx-auto">
      <div>
        <h1 className="text-2xl font-semibold mb-1">Dashboard</h1>
        <p className="text-sm text-muted-foreground">
          Welcome back. Here's your vault at a glance.
        </p>
      </div>

      {/* Onboarding Checklist */}
      {showChecklist && !allComplete && (
        <Card>
          <CardHeader className="pb-3">
            <div className="flex items-center justify-between">
              <CardTitle className="text-base">Getting Started</CardTitle>
              <Button
                variant="ghost"
                size="icon"
                className="size-7"
                onClick={handleDismiss}
              >
                <X className="size-3.5" />
              </Button>
            </div>
            <p className="text-xs text-muted-foreground">
              Complete these steps to get the most out of Context Vault.
            </p>
          </CardHeader>
          <CardContent className="space-y-3">
            {steps.map((step) => (
              <div key={step.id} className="flex items-start gap-3">
                <Checkbox
                  checked={step.completed}
                  onCheckedChange={() => toggleStep(step.id)}
                  className="mt-0.5"
                />
                <div className="flex-1">
                  <span className={step.completed ? "text-sm line-through text-muted-foreground" : "text-sm"}>
                    {step.label}
                  </span>
                  {step.id === "connect-claude" && !step.completed && (
                    <div className="mt-2 space-y-2">
                      <pre className="bg-muted p-3 rounded text-xs font-mono overflow-x-auto">
{`{
  "mcpServers": {
    "context-vault": {
      "url": "https://www.context-vault.com/mcp",
      "headers": {
        "Authorization": "Bearer YOUR_API_KEY"
      }
    }
  }
}`}
                      </pre>
                      <Button
                        variant="outline"
                        size="sm"
                        className="gap-1.5 text-xs"
                        onClick={copyMcpConfig}
                      >
                        {copiedConfig ? <Check className="size-3" /> : <Copy className="size-3" />}
                        Copy config
                      </Button>
                      <a
                        href="https://github.com/fellanH/context-mcp/blob/main/docs/distribution/connect-in-2-minutes.md"
                        target="_blank"
                        rel="noreferrer"
                        className="text-xs text-primary hover:underline inline-flex items-center gap-1"
                      >
                        Full setup guide
                        <ExternalLink className="size-3" />
                      </a>
                    </div>
                  )}
                  {step.id === "copy-api-key" && !step.completed && (
                    <div className="mt-1">
                      <Link to="/settings/api-keys" className="text-xs text-primary hover:underline inline-flex items-center gap-1">
                        Go to API Keys
                        <ExternalLink className="size-3" />
                      </Link>
                    </div>
                  )}
                </div>
              </div>
            ))}
          </CardContent>
        </Card>
      )}

      {/* Usage Overview */}
      {usageLoading ? (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {[1, 2, 3, 4].map((i) => (
            <Card key={i}>
              <CardHeader className="pb-2">
                <div className="h-4 w-16 bg-muted rounded animate-pulse" />
              </CardHeader>
              <CardContent className="space-y-2">
                <div className="h-8 w-20 bg-muted rounded animate-pulse" />
                <div className="h-1.5 bg-muted rounded animate-pulse" />
              </CardContent>
            </Card>
          ))}
        </div>
      ) : (
        <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4">
          {usageCards.map((card) => {
            const Icon = card.icon;
            const unlimited = isUnlimited(card.limit);
            const pct = !unlimited && card.limit > 0 ? (card.used / card.limit) * 100 : 0;
            const isWarning = !unlimited && pct >= 80;
            const isCritical = !unlimited && pct >= 100;

            return (
              <Card key={card.label}>
                <CardHeader className="pb-2">
                  <div className="flex items-center justify-between">
                    <CardTitle className="text-xs font-medium text-muted-foreground">
                      {card.label}
                    </CardTitle>
                    <Icon className="size-4 text-muted-foreground" />
                  </div>
                </CardHeader>
                <CardContent className="space-y-2">
                  <div className="flex items-baseline gap-1.5">
                    <span className={`text-2xl font-semibold ${isCritical ? "text-red-500" : isWarning ? "text-amber-500" : ""}`}>
                      {card.display}
                    </span>
                    {card.sub && (
                      <span className="text-xs text-muted-foreground">{card.sub}</span>
                    )}
                  </div>
                  {!unlimited && <UsageMeter used={card.used} limit={card.limit} />}
                  {isCritical && (
                    <Link to="/settings/billing" className="text-xs text-red-500 hover:underline">
                      Upgrade to increase limit
                    </Link>
                  )}
                </CardContent>
              </Card>
            );
          })}
        </div>
      )}

      {/* Recent Activity */}
      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-base">Recent Activity</CardTitle>
        </CardHeader>
        <CardContent>
          {entriesLoading ? (
            <div className="space-y-3 py-2">
              {[1, 2, 3].map((i) => (
                <div key={i} className="flex items-center justify-between py-2">
                  <div className="flex items-center gap-3">
                    <div className="h-4 w-48 bg-muted rounded animate-pulse" />
                    <div className="h-4 w-16 bg-muted rounded animate-pulse" />
                  </div>
                  <div className="h-4 w-12 bg-muted rounded animate-pulse" />
                </div>
              ))}
            </div>
          ) : entries.length === 0 ? (
            <p className="text-sm text-muted-foreground text-center py-6">
              No entries yet. Save your first entry to see activity here.
            </p>
          ) : (
            <div className="space-y-2">
              {entries
                .slice()
                .sort((a, b) => b.created.getTime() - a.created.getTime())
                .slice(0, 10)
                .map((entry) => (
                  <div
                    key={entry.id}
                    className="flex items-center justify-between py-2 border-b border-border last:border-0"
                  >
                    <div className="flex items-center gap-3 min-w-0">
                      <span className="text-sm font-medium truncate">{entry.title}</span>
                      <Badge
                        variant={
                          entry.category === "knowledge"
                            ? "default"
                            : entry.category === "entity"
                            ? "outline"
                            : "secondary"
                        }
                        className="text-[10px] shrink-0"
                      >
                        {entry.category}
                      </Badge>
                      <Badge variant="secondary" className="text-[10px] shrink-0">
                        {entry.kind}
                      </Badge>
                    </div>
                    <span className="text-xs text-muted-foreground whitespace-nowrap ml-4">
                      {formatRelativeTime(entry.created)}
                    </span>
                  </div>
                ))}
            </div>
          )}
        </CardContent>
      </Card>

      {/* Quick Actions */}
      <div className="flex flex-wrap gap-3">
        <Button variant="outline" asChild>
          <Link to="/search">
            <Search className="size-4 mr-1.5" />
            Search vault
          </Link>
        </Button>
        <Button variant="outline" asChild>
          <Link to="/vault/knowledge">
            <Plus className="size-4 mr-1.5" />
            New Entry
          </Link>
        </Button>
        <Button variant="outline" asChild>
          <Link to="/settings/data">
            <Upload className="size-4 mr-1.5" />
            Import data
          </Link>
        </Button>
      </div>
    </div>
  );
}
