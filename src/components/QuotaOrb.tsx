import type { CSSProperties, KeyboardEvent, MouseEvent } from "react";
import {
  formatPercent,
  formatResetAt,
  resolveQuotaTone,
  selectWeeklyQuotaWindow,
  WEEKLY_QUOTA_LABEL,
} from "../models/quota-view";
import type { Preferences, QuotaSnapshot } from "../types/ui";
import { Icon } from "./Icon";

interface QuotaOrbProps {
  snapshot: QuotaSnapshot;
  preferences: Preferences;
  modeBusy: boolean;
  onExpand: () => void;
  onOpenContextMenu: () => void;
}

const ORB_WATER_HEIGHT_PER_PERCENT = 0.64;

export function QuotaOrb({
  snapshot,
  preferences,
  modeBusy,
  onExpand,
  onOpenContextMenu,
}: QuotaOrbProps) {
  const quotaWindow = selectWeeklyQuotaWindow(snapshot, preferences.widget.selectedQuota);
  const hasValue =
    quotaWindow !== null &&
    (snapshot.status === "ok" || snapshot.status === "quotaReached" || snapshot.status === "stale");
  const tone = hasValue
    ? resolveQuotaTone(snapshot.status, quotaWindow.remainingPercent, preferences)
    : "neutral";
  const normalizedLevel = hasValue ? Math.min(100, Math.max(0, quotaWindow.remainingPercent)) : 0;
  const waterHeight = Math.round(normalizedLevel * ORB_WATER_HEIGHT_PER_PERCENT * 100) / 100;
  const waterStyle = hasValue
    ? ({
        "--quota-water-level": `${normalizedLevel}%`,
        "--quota-water-height": `${waterHeight}px`,
      } as CSSProperties)
    : undefined;

  const activate = (): void => {
    if (!modeBusy) {
      onExpand();
    }
  };

  const handleKeyDown = (event: KeyboardEvent<HTMLDivElement>): void => {
    if (event.key === "Enter" || event.key === " ") {
      event.preventDefault();
      activate();
    }
  };

  const handleContextMenu = (event: MouseEvent<HTMLDivElement>): void => {
    event.preventDefault();
    event.stopPropagation();
    onOpenContextMenu();
  };

  const accessibleValue = hasValue
    ? `${WEEKLY_QUOTA_LABEL}剩余 ${quotaWindow.remainingPercent}%`
    : snapshot.status === "loading"
      ? "正在读取额度"
      : "额度暂不可用";

  return (
    <div
      aria-busy={modeBusy}
      aria-haspopup="menu"
      aria-label={`${accessibleValue}，双击或按 Enter 展开卡片，右键打开菜单`}
      className="quota-orb"
      data-status={snapshot.status}
      data-tone={tone}
      onDoubleClick={activate}
      onContextMenu={handleContextMenu}
      onKeyDown={handleKeyDown}
      role="button"
      style={waterStyle}
      tabIndex={0}
    >
      <span aria-hidden="true" className="quota-orb__aurora" />
      {hasValue ? <span aria-hidden="true" className="quota-orb__water" /> : null}
      <svg aria-hidden="true" className="quota-orb__ring" viewBox="0 0 128 128">
        <circle className="quota-orb__track" cx="64" cy="64" pathLength="100" r="58" />
        {hasValue ? (
          <circle
            className="quota-orb__value"
            cx="64"
            cy="64"
            pathLength="100"
            r="58"
            strokeDasharray={`${quotaWindow.remainingPercent} 100`}
          />
        ) : null}
      </svg>
      <span className="quota-orb__content">
        {snapshot.status === "loading" ? <span className="skeleton skeleton--orb" /> : null}
        {hasValue ? (
          <>
            <span className="quota-orb__label">{WEEKLY_QUOTA_LABEL}</span>
            <strong aria-label={formatPercent(quotaWindow.remainingPercent)}>
              <span>{quotaWindow.remainingPercent}</span>
              <small>%</small>
            </strong>
            <span className="quota-orb__reset">{formatResetAt(quotaWindow.resetsAt)}</span>
            <span className="quota-orb__state">
              <span
                aria-hidden="true"
                className="quota-orb__status status-dot"
                data-status={snapshot.status}
              />
              <span>已展开额度卡片</span>
            </span>
          </>
        ) : null}
        {snapshot.status !== "loading" && !hasValue ? <Icon name="alert" size={22} /> : null}
      </span>
    </div>
  );
}
