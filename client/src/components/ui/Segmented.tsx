import type { ReactNode } from "react";
import { ToggleGroup } from "radix-ui";
import { cn } from "@/lib/cn";

export interface SegmentedOption<T extends string> {
  value: T;
  label: ReactNode;
  disabled?: boolean;
  title?: string;
}

/** Single-choice segmented control (theme mode, format). Always has a value. */
export function Segmented<T extends string>({
  value,
  onChange,
  options,
  label,
  size = "md",
  className,
}: {
  value: T;
  onChange: (value: T) => void;
  options: SegmentedOption<T>[];
  label: string;
  size?: "sm" | "md";
  className?: string;
}) {
  return (
    <ToggleGroup.Root
      type="single"
      value={value}
      onValueChange={(v) => v && onChange(v as T)}
      aria-label={label}
      className={cn("inline-flex rounded-control bg-surface p-0.5 hairline", className)}
    >
      {options.map((o) => (
        <ToggleGroup.Item
          key={o.value}
          value={o.value}
          disabled={o.disabled}
          title={o.title}
          className={cn(
            "inline-flex items-center justify-center gap-1.5 rounded-[calc(var(--radius-control)-2px)] font-medium text-fg-muted",
            "transition-[background-color,color] duration-150 hover:text-fg disabled:opacity-40",
            "data-[state=on]:bg-raised data-[state=on]:text-fg data-[state=on]:shadow-lift",
            size === "sm" ? "h-7 px-2.5 text-xs" : "h-8 px-3 text-sm",
          )}
        >
          {o.label}
        </ToggleGroup.Item>
      ))}
    </ToggleGroup.Root>
  );
}
