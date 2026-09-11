import { forwardRef, type ButtonHTMLAttributes, type HTMLAttributes } from "react";
import { cn } from "@/lib/cn";

const base =
  "inline-flex h-7 items-center gap-1.5 rounded-chip px-2.5 text-xs font-medium whitespace-nowrap";

/** Static label chip (language, count, state). */
export function Chip({ className, ...rest }: HTMLAttributes<HTMLSpanElement>) {
  return <span className={cn(base, "bg-surface text-fg-muted", className)} {...rest} />;
}

export interface ToggleChipProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  pressed: boolean;
}

/** Selectable chip; used for chapters and recent searches. */
export const ToggleChip = forwardRef<HTMLButtonElement, ToggleChipProps>(function ToggleChip(
  { pressed, className, type = "button", ...rest },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      aria-pressed={pressed}
      className={cn(
        base,
        "cursor-pointer transition-[background-color,color,transform] duration-100 active:scale-[0.97]",
        pressed ? "bg-accent text-accent-fg" : "bg-surface text-fg-muted hover:bg-raised hover:text-fg hairline",
        className,
      )}
      {...rest}
    />
  );
});
