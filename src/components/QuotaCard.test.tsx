import { fireEvent, render, screen, waitFor } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { createMockPreferences, createMockSnapshot, type MockScenario } from "../api/fixtures";
import { INITIAL_REFRESH_STATE, INITIAL_WINDOW_STATE } from "../models/defaults";
import type { QuotaBucket, QuotaSnapshot } from "../types/quota";
import { QuotaCard } from "./QuotaCard";

function renderCard(
  scenario: MockScenario,
  snapshotOverride?: QuotaSnapshot,
  settingsOpen = false,
) {
  const onRefresh = vi.fn();
  const onToggleAlwaysOnTop = vi.fn();
  const onToggleClickThrough = vi.fn();
  const onToggleLaunchAtLogin = vi.fn();
  const onChangeMode = vi.fn();
  const onChangeTheme = vi.fn();
  const onSettingsOpenChange = vi.fn();

  render(
    <QuotaCard
      feedback={null}
      onChangeMode={onChangeMode}
      onChangeTheme={onChangeTheme}
      onRefresh={onRefresh}
      onSettingsOpenChange={onSettingsOpenChange}
      onToggleAlwaysOnTop={onToggleAlwaysOnTop}
      onToggleClickThrough={onToggleClickThrough}
      onToggleLaunchAtLogin={onToggleLaunchAtLogin}
      pendingAction={null}
      preferences={createMockPreferences().preferences}
      refreshState={INITIAL_REFRESH_STATE}
      settingsOpen={settingsOpen}
      snapshot={snapshotOverride ?? createMockSnapshot(scenario)}
      windowState={INITIAL_WINDOW_STATE}
    />,
  );

  return {
    onRefresh,
    onToggleAlwaysOnTop,
    onToggleClickThrough,
    onToggleLaunchAtLogin,
    onChangeMode,
    onChangeTheme,
    onSettingsOpenChange,
  };
}

function requireFirstBucket(snapshot: QuotaSnapshot): QuotaBucket {
  const bucket = snapshot.buckets[0];
  if (bucket === undefined) {
    throw new Error("测试快照缺少额度桶");
  }
  return bucket;
}

describe("QuotaCard", () => {
  it("保留当前周额度主视觉并补齐参考项目的短周期与重置机会", () => {
    const base = createMockSnapshot("ok");
    const baseBucket = requireFirstBucket(base);
    const snapshot: QuotaSnapshot = {
      ...base,
      buckets: [
        {
          ...baseBucket,
          windows: [
            {
              slot: "secondary",
              kind: "shortTerm",
              label: "5 小时窗口",
              usedPercent: 26,
              remainingPercent: 74,
              windowDurationMins: 300,
              resetsAt: new Date(Date.now() + 3_600_000).toISOString(),
            },
            ...baseBucket.windows,
          ],
        },
        {
          limitId: "review",
          limitName: "代码审查",
          planType: "pro",
          windows: [
            {
              slot: "other",
              kind: "monthly",
              label: "月度审查额度",
              usedPercent: 12,
              remainingPercent: 88,
              windowDurationMins: 43_200,
              resetsAt: new Date(Date.now() + 86_400_000).toISOString(),
            },
          ],
          credits: null,
          rateLimitReachedType: null,
        },
      ],
    };

    renderCard("ok", snapshot);

    expect(screen.getByText("周额度")).toBeInTheDocument();
    expect(screen.getByText("5 小时额度")).toBeInTheDocument();
    expect(screen.getByText("74%")).toBeInTheDocument();
    expect(screen.getByText("重置机会")).toBeInTheDocument();
    expect(screen.getByText("1 次")).toBeInTheDocument();
    expect(screen.queryByText("月度审查额度")).not.toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "周额度剩余 96%" })).toHaveAttribute(
      "data-tone",
      "healthy",
    );
  });

  it("只有短周期时仍展示真实额度而不伪造周额度", () => {
    const base = createMockSnapshot("ok");
    const baseBucket = requireFirstBucket(base);
    const snapshot: QuotaSnapshot = {
      ...base,
      buckets: [
        {
          ...baseBucket,
          windows: [
            {
              slot: "primary",
              kind: "shortTerm",
              label: "5 小时窗口",
              usedPercent: 26,
              remainingPercent: 74,
              windowDurationMins: 300,
              resetsAt: new Date(Date.now() + 3_600_000).toISOString(),
            },
          ],
        },
      ],
    };

    renderCard("ok", snapshot);

    expect(screen.getByText("5 小时额度")).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "5 小时额度剩余 74%" })).toBeInTheDocument();
    expect(screen.queryByText("周额度")).not.toBeInTheDocument();
  });

  it("没有受支持周期时显示明确空状态", () => {
    const base = createMockSnapshot("ok");
    const baseBucket = requireFirstBucket(base);
    const snapshot: QuotaSnapshot = {
      ...base,
      buckets: [
        {
          ...baseBucket,
          windows: [
            {
              slot: "primary",
              kind: "monthly",
              label: "月度额度",
              usedPercent: 20,
              remainingPercent: 80,
              windowDurationMins: 43_200,
              resetsAt: new Date(Date.now() + 86_400_000).toISOString(),
            },
          ],
        },
      ],
    };

    renderCard("ok", snapshot);

    expect(screen.getByText("暂无可用额度")).toBeInTheDocument();
    expect(screen.queryByRole("progressbar")).not.toBeInTheDocument();
  });

  it("使用提醒语义展示低额度", () => {
    renderCard("warning");

    expect(screen.getByRole("progressbar", { name: "周额度剩余 22%" })).toHaveAttribute(
      "data-tone",
      "warning",
    );
  });

  it("使用危险语义展示触顶额度", () => {
    renderCard("danger");

    expect(screen.getByRole("progressbar", { name: "周额度剩余 7%" })).toHaveAttribute(
      "data-tone",
      "danger",
    );
  });

  it("旧数据保留最后成功值并明确标记", () => {
    renderCard("stale");

    expect(screen.getByLabelText("96%")).toBeInTheDocument();
    expect(screen.getByText(/数据可能已过期/)).toBeInTheDocument();
    expect(screen.getByRole("progressbar", { name: "周额度剩余 96%" })).toHaveAttribute(
      "data-tone",
      "neutral",
    );
  });

  it("加载时保留布局且不伪造 0%", () => {
    renderCard("loading");

    expect(screen.getByRole("status", { name: "正在读取额度" })).toBeInTheDocument();
    expect(screen.queryByText("0%")).not.toBeInTheDocument();
  });

  it("错误状态给出单一重试动作", () => {
    const actions = renderCard("error");

    expect(screen.getByText("未找到可用的 Codex")).toBeInTheDocument();
    expect(
      screen.getByText(/ChatGPT 桌面应用（包含 Codex）/),
    ).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: "重新读取" }));
    expect(actions.onRefresh).toHaveBeenCalledTimes(1);
  });

  it("置顶、设置和模式切换都提供可操作入口", () => {
    const actions = renderCard("ok");
    const card = screen.getByLabelText(/QuotaGlance 额度卡片/);

    fireEvent.click(screen.getByRole("button", { name: "取消窗口置顶" }));
    fireEvent.click(screen.getByRole("button", { name: "打开设置" }));
    fireEvent.doubleClick(card);
    fireEvent.keyDown(card, { key: "Enter" });

    expect(actions.onToggleAlwaysOnTop).toHaveBeenCalledTimes(1);
    expect(actions.onSettingsOpenChange).toHaveBeenCalledWith(true);
    expect(actions.onChangeMode).toHaveBeenNthCalledWith(1, "orb");
    expect(actions.onChangeMode).toHaveBeenNthCalledWith(2, "orb");
  });

  it("设置面板提供可持久化的主题切换入口", () => {
    const actions = renderCard("ok", undefined, true);

    expect(screen.getAllByRole("radio")).toHaveLength(7);
    expect(screen.getByRole("radio", { name: "跟随系统主题" })).toHaveAttribute(
      "aria-checked",
      "true",
    );
    fireEvent.click(screen.getByRole("radio", { name: "极光主题" }));
    fireEvent.click(screen.getByRole("radio", { name: "石墨主题" }));
    fireEvent.click(screen.getByRole("radio", { name: "纸白主题" }));
    fireEvent.click(screen.getByRole("radio", { name: "日落珊瑚主题" }));
    fireEvent.click(screen.getByRole("radio", { name: "蜂蜜琥珀主题" }));
    fireEvent.click(screen.getByRole("radio", { name: "玫瑰铜夜主题" }));
    fireEvent.click(screen.getByRole("switch", { name: "登录时启动" }));

    expect(actions.onChangeTheme).toHaveBeenNthCalledWith(1, "aurora");
    expect(actions.onChangeTheme).toHaveBeenNthCalledWith(2, "graphite");
    expect(actions.onChangeTheme).toHaveBeenNthCalledWith(3, "paper");
    expect(actions.onChangeTheme).toHaveBeenNthCalledWith(4, "sunset");
    expect(actions.onChangeTheme).toHaveBeenNthCalledWith(5, "honey");
    expect(actions.onChangeTheme).toHaveBeenNthCalledWith(6, "rose");
    expect(actions.onToggleLaunchAtLogin).toHaveBeenCalledTimes(1);
  });

  it("Escape 关闭设置后将焦点归还设置按钮", async () => {
    const { rerender } = render(
      <QuotaCard
        feedback={null}
        onChangeMode={vi.fn()}
        onChangeTheme={vi.fn()}
        onRefresh={vi.fn()}
        onSettingsOpenChange={vi.fn()}
        onToggleAlwaysOnTop={vi.fn()}
        onToggleClickThrough={vi.fn()}
        onToggleLaunchAtLogin={vi.fn()}
        pendingAction={null}
        preferences={createMockPreferences().preferences}
        refreshState={INITIAL_REFRESH_STATE}
        settingsOpen
        snapshot={createMockSnapshot("ok")}
        windowState={INITIAL_WINDOW_STATE}
      />,
    );

    const onSettingsOpenChange = vi.fn((open: boolean) => {
      if (!open) {
        rerender(
          <QuotaCard
            feedback={null}
            onChangeMode={vi.fn()}
            onChangeTheme={vi.fn()}
            onRefresh={vi.fn()}
            onSettingsOpenChange={onSettingsOpenChange}
            onToggleAlwaysOnTop={vi.fn()}
            onToggleClickThrough={vi.fn()}
            onToggleLaunchAtLogin={vi.fn()}
            pendingAction={null}
            preferences={createMockPreferences().preferences}
            refreshState={INITIAL_REFRESH_STATE}
            settingsOpen={false}
            snapshot={createMockSnapshot("ok")}
            windowState={INITIAL_WINDOW_STATE}
          />,
        );
      }
    });

    rerender(
      <QuotaCard
        feedback={null}
        onChangeMode={vi.fn()}
        onChangeTheme={vi.fn()}
        onRefresh={vi.fn()}
        onSettingsOpenChange={onSettingsOpenChange}
        onToggleAlwaysOnTop={vi.fn()}
        onToggleClickThrough={vi.fn()}
        onToggleLaunchAtLogin={vi.fn()}
        pendingAction={null}
        preferences={createMockPreferences().preferences}
        refreshState={INITIAL_REFRESH_STATE}
        settingsOpen
        snapshot={createMockSnapshot("ok")}
        windowState={INITIAL_WINDOW_STATE}
      />,
    );

    fireEvent.keyDown(screen.getByRole("dialog", { name: "设置" }), { key: "Escape" });

    expect(onSettingsOpenChange).toHaveBeenCalledWith(false);
    await waitFor(() => expect(screen.getByRole("button", { name: "打开设置" })).toHaveFocus());
  });
});
