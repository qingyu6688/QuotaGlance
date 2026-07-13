import { fireEvent, render, screen } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { createMockPreferences, createMockSnapshot } from "../api/fixtures";
import { QuotaOrb } from "./QuotaOrb";

describe("QuotaOrb", () => {
  it("显示周额度并按剩余比例设置动态水位", () => {
    render(
      <QuotaOrb
        modeBusy={false}
        onExpand={vi.fn()}
        onOpenContextMenu={vi.fn()}
        preferences={createMockPreferences().preferences}
        snapshot={createMockSnapshot("ok")}
      />,
    );

    const orb = screen.getByRole("button", { name: /周额度剩余 96%/ });
    expect(orb).toHaveStyle(
      "--quota-water-level: 96%",
    );
    expect(orb).toHaveStyle("--quota-water-height: 61.44px");
    expect(orb).toHaveTextContent("周额度");
    expect(orb).toHaveTextContent("96%");
    expect(orb).toHaveTextContent(/\d+ 月 \d+ 日重置/);
    expect(orb).toHaveTextContent("已展开额度卡片");
  });

  it("支持 Enter、Space 和双击展开", () => {
    const onExpand = vi.fn();
    render(
      <QuotaOrb
        modeBusy={false}
        onExpand={onExpand}
        onOpenContextMenu={vi.fn()}
        preferences={createMockPreferences().preferences}
        snapshot={createMockSnapshot("ok")}
      />,
    );

    const orb = screen.getByRole("button", { name: /双击或按 Enter 展开卡片/ });
    fireEvent.keyDown(orb, { key: "Enter" });
    fireEvent.keyDown(orb, { key: " " });
    fireEvent.doubleClick(orb);

    expect(onExpand).toHaveBeenCalledTimes(3);
  });

  it("仅在左键越过移动阈值后启动一次拖拽，并抑制同一手势的双击", () => {
    const onExpand = vi.fn();
    const onStartDragging = vi.fn();
    const commonProps = {
      onExpand,
      onOpenContextMenu: vi.fn(),
      onStartDragging,
      preferences: createMockPreferences().preferences,
      snapshot: createMockSnapshot("ok"),
    };
    const { rerender } = render(<QuotaOrb {...commonProps} modeBusy={false} />);

    const orb = screen.getByRole("button", { name: /按住左键拖动/ });
    fireEvent.mouseDown(orb, { button: 0, buttons: 1, clientX: 10, clientY: 10, detail: 1 });
    fireEvent.mouseMove(orb, { buttons: 1, clientX: 12, clientY: 12 });
    expect(onStartDragging).not.toHaveBeenCalled();

    fireEvent.mouseMove(orb, { buttons: 1, clientX: 16, clientY: 10 });
    fireEvent.mouseMove(orb, { buttons: 1, clientX: 24, clientY: 10 });
    fireEvent.mouseUp(orb, { button: 0 });
    expect(onStartDragging).toHaveBeenCalledTimes(1);

    fireEvent.doubleClick(orb);
    expect(onExpand).not.toHaveBeenCalled();

    fireEvent.mouseDown(orb, { button: 2, buttons: 2, clientX: 10, clientY: 10 });
    fireEvent.mouseMove(orb, { buttons: 2, clientX: 30, clientY: 10 });
    expect(onStartDragging).toHaveBeenCalledTimes(1);

    rerender(<QuotaOrb {...commonProps} modeBusy />);
    fireEvent.mouseDown(orb, { button: 0, buttons: 1, clientX: 10, clientY: 10 });
    fireEvent.mouseMove(orb, { buttons: 1, clientX: 30, clientY: 10 });
    expect(onStartDragging).toHaveBeenCalledTimes(1);
  });

  it("右键阻止 WebView 默认菜单并打开精简菜单", () => {
    const onOpenContextMenu = vi.fn();
    render(
      <QuotaOrb
        modeBusy={false}
        onExpand={vi.fn()}
        onOpenContextMenu={onOpenContextMenu}
        preferences={createMockPreferences().preferences}
        snapshot={createMockSnapshot("ok")}
      />,
    );

    const orb = screen.getByRole("button", { name: /右键打开菜单/ });
    expect(fireEvent.contextMenu(orb)).toBe(false);
    expect(onOpenContextMenu).toHaveBeenCalledTimes(1);
  });

  it("错误时不显示虚假百分比", () => {
    render(
      <QuotaOrb
        modeBusy={false}
        onExpand={vi.fn()}
        onOpenContextMenu={vi.fn()}
        preferences={createMockPreferences().preferences}
        snapshot={createMockSnapshot("error")}
      />,
    );

    expect(screen.getByRole("button", { name: /额度暂不可用/ })).toBeInTheDocument();
    expect(screen.queryByText("0%")).not.toBeInTheDocument();
  });
});
