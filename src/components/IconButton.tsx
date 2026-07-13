import type { ButtonHTMLAttributes, Ref } from "react";
import { Icon, type IconName } from "./Icon";

interface IconButtonProps
  extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, "children" | "aria-label"> {
  icon: IconName;
  label: string;
  busy?: boolean;
  buttonRef?: Ref<HTMLButtonElement>;
}

export function IconButton({
  icon,
  label,
  busy = false,
  buttonRef,
  className = "",
  ...buttonProps
}: IconButtonProps) {
  const classes = ["icon-button", busy ? "icon-button--busy" : "", className]
    .filter(Boolean)
    .join(" ");

  return (
    <button
      aria-busy={busy}
      aria-label={label}
      className={classes}
      ref={buttonRef}
      title={label}
      type="button"
      {...buttonProps}
    >
      <Icon name={icon} />
    </button>
  );
}
