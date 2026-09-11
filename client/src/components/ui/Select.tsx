import type { ReactNode } from "react";
import { Select as RadixSelect } from "radix-ui";
import { CaretDown, Check } from "@phosphor-icons/react";
import { cn } from "@/lib/cn";

export interface SelectOption<T extends string> {
  value: T;
  label: ReactNode;
  /** Shown in the trigger when selected; defaults to `label`. */
  triggerLabel?: ReactNode;
  disabled?: boolean;
  description?: ReactNode;
}

export function Select<T extends string>({
  value,
  onChange,
  options,
  label,
  className,
  contentClassName,
}: {
  value: T;
  onChange: (value: T) => void;
  options: SelectOption<T>[];
  label: string;
  className?: string;
  contentClassName?: string;
}) {
  const current = options.find((o) => o.value === value);
  return (
    <RadixSelect.Root value={value} onValueChange={(v) => onChange(v as T)}>
      <RadixSelect.Trigger
        aria-label={label}
        className={cn(
          "inline-flex h-10 items-center justify-between gap-2 rounded-control bg-raised px-3 text-sm text-fg hairline",
          "transition-shadow data-[state=open]:shadow-[inset_0_0_0_1.5px_var(--color-accent)]",
          className,
        )}
      >
        <span className="truncate">{current?.triggerLabel ?? current?.label}</span>
        <RadixSelect.Icon className="shrink-0 text-fg-faint">
          <CaretDown size={14} />
        </RadixSelect.Icon>
      </RadixSelect.Trigger>
      <RadixSelect.Portal>
        <RadixSelect.Content
          position="popper"
          sideOffset={6}
          className={cn(
            "z-30 max-h-[min(24rem,var(--radix-select-content-available-height))] min-w-(--radix-select-trigger-width) overflow-hidden rounded-panel bg-raised p-1 shadow-float hairline",
            contentClassName,
          )}
        >
          <RadixSelect.Viewport>
            {options.map((o) => (
              <RadixSelect.Item
                key={o.value}
                value={o.value}
                disabled={o.disabled}
                className={cn(
                  "relative flex cursor-pointer select-none flex-col gap-0.5 rounded-control py-2 pr-8 pl-3 text-sm outline-none",
                  "data-[highlighted]:bg-surface data-[disabled]:cursor-not-allowed data-[disabled]:opacity-50",
                )}
              >
                <RadixSelect.ItemText>{o.label}</RadixSelect.ItemText>
                {o.description && <span className="text-xs text-fg-faint">{o.description}</span>}
                <RadixSelect.ItemIndicator className="absolute top-2.5 right-2.5 text-accent">
                  <Check size={14} weight="bold" />
                </RadixSelect.ItemIndicator>
              </RadixSelect.Item>
            ))}
          </RadixSelect.Viewport>
        </RadixSelect.Content>
      </RadixSelect.Portal>
    </RadixSelect.Root>
  );
}
