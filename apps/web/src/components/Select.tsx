import * as Select from "@radix-ui/react-select";
import React from "react";

export type SelectOption = {
  value: string;
  label: string;
  disabled?: boolean;
};

type SelectFieldProps = {
  value?: string;
  onValueChange: (value: string) => void;
  options: SelectOption[];
  placeholder?: string;
  ariaLabel?: string;
  size?: "default" | "compact" | "inline";
  className?: string;
  disabled?: boolean;
};

export function SelectField({
  value,
  onValueChange,
  options,
  placeholder = "Select",
  ariaLabel,
  size = "default",
  className,
  disabled = false
}: SelectFieldProps) {
  const sizeClass =
    size === "compact" ? "select-compact" : size === "inline" ? "select-inline" : "";

  return (
    <Select.Root value={value} onValueChange={onValueChange} disabled={disabled}>
      <Select.Trigger
        className={`select-trigger ${sizeClass} ${className ?? ""}`.trim()}
        aria-label={ariaLabel ?? placeholder}
      >
        <Select.Value placeholder={placeholder} />
        <Select.Icon className="select-icon" aria-hidden="true">
          <svg viewBox="0 0 16 16" width="16" height="16" focusable="false" aria-hidden="true">
            <path
              d="M4 6l4 4 4-4"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.8"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
        </Select.Icon>
      </Select.Trigger>
      <Select.Portal>
        <Select.Content className="select-content" position="popper" sideOffset={6}>
          <Select.Viewport className="select-viewport">
            {options.map((option) => (
              <Select.Item
                key={option.value}
                value={option.value}
                disabled={option.disabled}
                className="select-item"
              >
                <Select.ItemText>{option.label}</Select.ItemText>
                <Select.ItemIndicator className="select-item-indicator">
                  <svg viewBox="0 0 16 16" width="12" height="12" aria-hidden="true">
                    <path
                      d="M3.3 8.4l2.7 2.8 6-6.3"
                      fill="none"
                      stroke="currentColor"
                      strokeWidth="1.8"
                      strokeLinecap="round"
                      strokeLinejoin="round"
                    />
                  </svg>
                </Select.ItemIndicator>
              </Select.Item>
            ))}
          </Select.Viewport>
        </Select.Content>
      </Select.Portal>
    </Select.Root>
  );
}
