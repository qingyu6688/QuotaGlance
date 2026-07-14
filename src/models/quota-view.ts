import type {
  Preferences,
  QuotaBucket,
  QuotaSnapshot,
  QuotaStatus,
  QuotaWindow,
  SelectedQuota,
} from "../types/quota";

export type QuotaTone = "healthy" | "warning" | "danger" | "neutral";

export const WEEKLY_QUOTA_LABEL = "周额度";
export const SHORT_TERM_QUOTA_LABEL = "短周期额度";

export interface QuotaPresentation {
  bucket: QuotaBucket | null;
  primaryWindow: QuotaWindow | null;
  weeklyWindow: QuotaWindow | null;
  resetCreditCount: number | null;
  resetCreditExpirations: string[];
}

function selectPreferredBucket(
  snapshot: QuotaSnapshot,
  selectedQuota: SelectedQuota,
): QuotaBucket | null {
  if (selectedQuota.limitId !== null) {
    const selected = snapshot.buckets.find((bucket) => bucket.limitId === selectedQuota.limitId);
    if (selected !== undefined) {
      return selected;
    }
  }

  return (
    snapshot.buckets.find(
      (bucket) =>
        bucket.limitId.toLowerCase() === "codex" &&
        bucket.windows.some(isSupportedQuotaWindow),
    ) ??
    snapshot.buckets.find((bucket) => bucket.windows.some(isSupportedQuotaWindow)) ??
    null
  );
}

function isSupportedQuotaWindow(quotaWindow: QuotaWindow): boolean {
  return quotaWindow.kind === "shortTerm" || quotaWindow.kind === "weekly";
}

export function resolveQuotaPresentation(
  snapshot: QuotaSnapshot,
  selectedQuota: SelectedQuota,
): QuotaPresentation {
  const bucket = selectPreferredBucket(snapshot, selectedQuota);
  const primaryWindow =
    bucket?.windows.find((quotaWindow) => quotaWindow.kind === "shortTerm") ?? null;
  const weeklyWindow =
    bucket?.windows.find((quotaWindow) => quotaWindow.kind === "weekly") ?? null;
  const resetCreditExpirations = Array.from(
    new Set(
      (snapshot.bankedResets?.details ?? [])
        .map((detail) => detail.expiresAt)
        .filter((value): value is string => value !== null),
    ),
  ).sort();

  return {
    bucket,
    primaryWindow,
    weeklyWindow,
    resetCreditCount: snapshot.bankedResets?.availableCount ?? null,
    resetCreditExpirations,
  };
}

export function selectWeeklyQuotaWindow(
  snapshot: QuotaSnapshot,
  selectedQuota: SelectedQuota,
): QuotaWindow | null {
  return resolveQuotaPresentation(snapshot, selectedQuota).weeklyWindow;
}

export function resolveQuotaWindowLabel(quotaWindow: QuotaWindow): string {
  if (quotaWindow.kind === "weekly") {
    return WEEKLY_QUOTA_LABEL;
  }

  const hours = quotaWindow.windowDurationMins / 60;
  if (Number.isInteger(hours) && hours > 0 && hours < 24) {
    return `${hours} 小时额度`;
  }
  return SHORT_TERM_QUOTA_LABEL;
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

export function formatExpirationAt(value: string): string {
  const expiration = Date.parse(value);
  if (!Number.isFinite(expiration)) {
    return "到期时间未知";
  }

  return new Intl.DateTimeFormat("zh-CN", {
    month: "numeric",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  }).format(expiration);
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
