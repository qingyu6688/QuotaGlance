import type { Theme } from "../types/ui";
import { Icon } from "./Icon";

interface ThemeControlProps {
  value: Theme;
  disabled: boolean;
  onChange: (theme: Theme) => void;
}

const THEME_OPTIONS: ReadonlyArray<{
  value: Theme;
  label: string;
  accessibleLabel: string;
}> = [
  { value: "system", label: "跟随", accessibleLabel: "跟随系统主题" },
  { value: "aurora", label: "极光", accessibleLabel: "极光主题" },
  { value: "graphite", label: "石墨", accessibleLabel: "石墨主题" },
  { value: "paper", label: "纸白", accessibleLabel: "纸白主题" },
  { value: "sunset", label: "日落", accessibleLabel: "日落珊瑚主题" },
  { value: "honey", label: "琥珀", accessibleLabel: "蜂蜜琥珀主题" },
  { value: "rose", label: "玫瑰铜", accessibleLabel: "玫瑰铜夜主题" },
];

export function ThemeControl({ value, disabled, onChange }: ThemeControlProps) {
  return (
    <section className="theme-setting" aria-labelledby="theme-setting-title">
      <div className="theme-setting__heading">
        <Icon name="palette" size={17} />
        <strong id="theme-setting-title">主题</strong>
      </div>
      <div aria-busy={disabled} aria-label="外观主题" className="theme-control" role="radiogroup">
        {THEME_OPTIONS.map((option) => (
          <button
            aria-checked={value === option.value}
            aria-label={option.accessibleLabel}
            disabled={disabled}
            key={option.value}
            onClick={() => onChange(option.value)}
            role="radio"
            type="button"
          >
            <span
              aria-hidden="true"
              className="theme-control__swatch"
              data-theme-option={option.value}
            >
              {option.value === "system" ? <Icon name="monitor" size={22} /> : null}
            </span>
            {value === option.value ? (
              <span aria-hidden="true" className="theme-control__selected">
                <Icon name="check" size={11} />
              </span>
            ) : null}
            <span>{option.label}</span>
          </button>
        ))}
      </div>
    </section>
  );
}
