import type { QuotaTone } from "../models/quota-view";

interface ProgressBarProps {
  label: string;
  value: number;
  tone: QuotaTone;
}

export function ProgressBar({ label, value, tone }: ProgressBarProps) {
  return (
    <div
      aria-label={`${label}剩余 ${value}%`}
      aria-valuemax={100}
      aria-valuemin={0}
      aria-valuenow={value}
      className="progress-bar"
      data-tone={tone}
      role="progressbar"
    >
      <span className="progress-bar__value" style={{ width: `${value}%` }} />
    </div>
  );
}
