import { Progress } from "./ui/progress";
import { cn } from "./ui/utils";

interface UsageMeterProps {
  used: number;
  limit: number;
  label?: string;
  unit?: string;
  className?: string;
}

export function UsageMeter({ used, limit, label, unit, className }: UsageMeterProps) {
  const pct = limit > 0 ? (used / limit) * 100 : 0;
  const isWarning = pct >= 80;
  const isCritical = pct >= 100;

  return (
    <div className={cn("space-y-1.5", className)}>
      {label && (
        <div className="flex items-center justify-between text-xs">
          <span className="text-muted-foreground">{label}</span>
          <span className={cn(
            "font-mono",
            isCritical && "text-red-500",
            isWarning && !isCritical && "text-amber-500",
          )}>
            {used}{unit ? ` ${unit}` : ""} / {limit}{unit ? ` ${unit}` : ""}
          </span>
        </div>
      )}
      <Progress
        value={Math.min(pct, 100)}
        className={cn(
          "h-1.5",
          isCritical && "[&>div]:bg-red-500",
          isWarning && !isCritical && "[&>div]:bg-amber-500",
        )}
      />
    </div>
  );
}
