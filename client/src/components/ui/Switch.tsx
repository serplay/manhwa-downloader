import { useId, type ReactNode } from "react";
import { Switch as RadixSwitch } from "radix-ui";
import { cn } from "@/lib/cn";

export function Switch({
  checked,
  onCheckedChange,
  label,
  description,
  className,
}: {
  checked: boolean;
  onCheckedChange: (v: boolean) => void;
  label: ReactNode;
  description?: ReactNode;
  className?: string;
}) {
  const id = useId();
  return (
    <div className={cn("flex items-start justify-between gap-3", className)}>
      <label htmlFor={id} className="flex min-w-0 cursor-pointer flex-col gap-0.5">
        <span className="text-sm font-medium">{label}</span>
        {description && <span className="text-xs text-fg-muted">{description}</span>}
      </label>
      <RadixSwitch.Root
        id={id}
        checked={checked}
        onCheckedChange={onCheckedChange}
        className={cn(
          "relative h-6 w-10 shrink-0 rounded-full bg-line-strong transition-colors",
          "data-[state=checked]:bg-accent",
        )}
      >
        <RadixSwitch.Thumb className="block size-5 translate-x-0.5 rounded-full bg-raised shadow-lift transition-transform duration-150 ease-spring data-[state=checked]:translate-x-[18px]" />
      </RadixSwitch.Root>
    </div>
  );
}
