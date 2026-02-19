import type { OnboardingStep } from "./types";

const STORAGE_KEY = "context-vault-onboarding";
const DISMISSED_KEY = "context-vault-onboarding-dismissed";

const defaultSteps: OnboardingStep[] = [
  { id: "create-account", label: "Create account", completed: false },
  { id: "copy-api-key", label: "Copy your API key", completed: false },
  { id: "connect-claude", label: "Connect to Claude Code", completed: false, description: "Add the MCP config to your Claude Code settings" },
  { id: "first-entry", label: "Save your first entry", completed: false },
];

function loadSteps(): OnboardingStep[] {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) return JSON.parse(raw);
  } catch {}
  return defaultSteps.map((s) => ({ ...s }));
}

function saveSteps(steps: OnboardingStep[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(steps));
}

export function getOnboardingSteps(isAuthenticated: boolean, entriesUsed: number): OnboardingStep[] {
  const steps = loadSteps();

  // Auto-complete based on app state
  let changed = false;
  for (const step of steps) {
    if (step.id === "create-account" && isAuthenticated && !step.completed) {
      step.completed = true;
      changed = true;
    }
    if (step.id === "first-entry" && entriesUsed > 0 && !step.completed) {
      step.completed = true;
      changed = true;
    }
  }

  if (changed) saveSteps(steps);
  return steps;
}

export function toggleOnboardingStep(id: string): OnboardingStep[] {
  const steps = loadSteps();
  const step = steps.find((s) => s.id === id);
  if (step) {
    step.completed = !step.completed;
    saveSteps(steps);
  }
  return steps;
}

export function isOnboardingDismissed(): boolean {
  return localStorage.getItem(DISMISSED_KEY) === "true";
}

export function dismissOnboarding() {
  localStorage.setItem(DISMISSED_KEY, "true");
}
