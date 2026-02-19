import type { OnboardingStep } from "./types";

const DISMISSED_KEY = "context-vault-onboarding-dismissed";

interface OnboardingInputs {
  isAuthenticated: boolean;
  isLocalMode: boolean;
  entriesUsed: number;
  hasApiKey: boolean;
  hasMcpActivity: boolean;
}

export function getOnboardingSteps({
  isAuthenticated,
  isLocalMode,
  entriesUsed,
  hasApiKey,
  hasMcpActivity,
}: OnboardingInputs): OnboardingStep[] {
  if (isLocalMode) {
    return [
      {
        id: "connect-local",
        label: "Connect local vault",
        completed: isAuthenticated,
        description: "Local mode is active for this dashboard session",
      },
      {
        id: "first-entry",
        label: "Save your first entry",
        completed: entriesUsed > 0,
      },
    ];
  }

  return [
    { id: "create-account", label: "Create account", completed: isAuthenticated },
    { id: "copy-api-key", label: "Copy your API key", completed: hasApiKey },
    {
      id: "connect-claude",
      label: "Connect to Claude Code",
      completed: hasMcpActivity,
      description: "Detected from API key usage",
    },
    { id: "first-entry", label: "Save your first entry", completed: entriesUsed > 0 },
  ];
}

export function isOnboardingDismissed(): boolean {
  return localStorage.getItem(DISMISSED_KEY) === "true";
}

export function dismissOnboarding() {
  localStorage.setItem(DISMISSED_KEY, "true");
}
