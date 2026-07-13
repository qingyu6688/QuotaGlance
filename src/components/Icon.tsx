import type { ComponentType } from "react";
import {
  IconAlertTriangle,
  IconCalendar,
  IconCheck,
  IconClock,
  IconDeviceDesktop,
  IconInfoCircle,
  IconMouse,
  IconPalette,
  IconPin,
  IconPower,
  IconRefresh,
  IconSettings,
  IconX,
  type IconProps as TablerIconProps,
} from "@tabler/icons-react";

export type IconName =
  | "refresh"
  | "pin"
  | "settings"
  | "clock"
  | "calendar"
  | "close"
  | "alert"
  | "info"
  | "power"
  | "palette"
  | "monitor"
  | "check"
  | "mouse";

interface IconProps {
  name: IconName;
  size?: number;
}

const ICONS: Readonly<Record<IconName, ComponentType<TablerIconProps>>> = {
  refresh: IconRefresh,
  pin: IconPin,
  settings: IconSettings,
  clock: IconClock,
  calendar: IconCalendar,
  close: IconX,
  alert: IconAlertTriangle,
  info: IconInfoCircle,
  power: IconPower,
  palette: IconPalette,
  monitor: IconDeviceDesktop,
  check: IconCheck,
  mouse: IconMouse,
};

export function Icon({ name, size = 20 }: IconProps) {
  const Component = ICONS[name];

  return <Component aria-hidden="true" className="icon" size={size} stroke={1.75} />;
}
