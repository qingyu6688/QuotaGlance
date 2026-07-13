import type {
  Preferences,
  QuotaSnapshot,
  QuotaStatus,
  QuotaWindow,
  SelectedQuota,
} from "../types/quota";

export type QuotaTone = "healthy" | "warning" | "danger" | "neutral";

export const WEEKLY_QUOTA_LABEL = "周额度";

export function selectWeeklyQuotaWindow(
  snapshot: QuotaSnapshot,
  selectedQuota: SelectedQuota,
): QuotaWindow | null {
  let firstWeeklyWindow: QuotaWindow | null = null;

  for (const bucket of snapshot.buckets) {
    for (const quotaWindow of bucket.windows) {
      if (quotaWindow.kind !== "weekly") {
        continue;
      }

      firstWeeklyWindow ??= quotaWindow;
      if (selectedQuota.limitId === null || bucket.limitId === selectedQuota.limitId) {
        return quotaWindow;
      }
    }
  }

  return firstWeeklyWindow;
}

export function resolvePlanLabel(snapshot: QuotaSnapshot): string {
  const planType = snapshot.buckets.find((bucket) => bucket.planType !== null)?.planType;
  const value = planType ?? snapshot.auth.planType;

  if (value === null || value.trim() === "") {
    return "Codex";
  }

  const knownPlans: Readonly<Record<string, string>> = {
    free: "Free",
    plus: "Plus",
    pro: "Pro",
    team: "Team",
    business: "Business",
    enterprise: "Enterprise",
    edu: "Edu",
  };
  return `Codex · ${knownPlans[value.toLowerCase()] ?? value}`;
}

export function resolveQuotaTone(
  status: QuotaStatus,
  remainingPercent: number,
  preferences: Preferences,
): QuotaTone {
  if (status === "stale") {
    return "neutral";
  }
  if (status === "quotaReached") {
    return "danger";
  }
  if (remainingPercent <= preferences.notifications.criticalRemainingPercent) {
    return "danger";
  }
  if (remainingPercent <= preferences.notifications.warningRemainingPercent) {
    return "warning";
  }
  return "healthy";
}

export function formatPercent(value: number): string {
  return `${value}%`;
}

export function formatResetAt(value: string, now = Date.now()): string {
  const resetAt = Date.parse(value);
  if (!Number.isFinite(resetAt)) {
    return "重置时间未知";
  }

  const remainingMinutes = Math.ceil((resetAt - now) / 60_000);
  if (remainingMinutes > 0 && remainingMinutes < 1_440) {
    const hours = Math.floor(remainingMinutes / 60);
    const minutes = remainingMinutes % 60;
    if (hours === 0) {
      return `${minutes} 分后重置`;
    }
    if (minutes === 0) {
      return `${hours} 小时后重置`;
    }
    return `${hours} 小时 ${minutes} 分后重置`;
  }

  const date = new Date(resetAt);
  return `${date.getMonth() + 1} 月 ${date.getDate()} 日重置`;
}

export function formatFreshness(snapshot: QuotaSnapshot, now = Date.now()): string {
  const source = snapshot.status === "stale" ? snapshot.lastGoodAt : snapshot.fetchedAt;
  if (source === null) {
    return snapshot.status === "loading" ? "正在读取额度" : "尚无可用数据";
  }

  const timestamp = Date.parse(source);
  if (!Number.isFinite(timestamp)) {
    return "更新时间未知";
  }

  const elapsedMinutes = Math.max(0, Math.floor((now - timestamp) / 60_000));
  const prefix = snapshot.status === "stale" ? "最后成功：" : "最后更新：";
  if (elapsedMinutes < 1) {
    return `${prefix}刚刚`;
  }
  if (elapsedMinutes < 60) {
    return `${prefix}${elapsedMinutes} 分钟前`;
  }

  const elapsedHours = Math.floor(elapsedMinutes / 60);
  return `${prefix}${elapsedHours} 小时前`;
}
