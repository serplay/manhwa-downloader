import { forwardRef, type ButtonHTMLAttributes } from "react";
import { cn } from "@/lib/cn";

export interface IconButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  /** Accessible name; icon-only buttons must have one. */
  label: string;
  size?: "sm" | "md";
}

export const IconButton = forwardRef<HTMLButtonElement, IconButtonProps>(function IconButton(
  { className, label, size = "md", type = "button", children, ...rest },
  ref,
) {
  return (
    <button
      ref={ref}
      type={type}
      aria-label={label}
      title={label}
      className={cn(
        "inline-flex shrink-0 items-center justify-center rounded-control text-fg-muted",
        "transition-[background-color,color,transform] duration-150 hover:bg-surface hover:text-fg active:scale-[0.96]",
        "disabled:pointer-events-none disabled:opacity-50",
        size === "sm" ? "size-8" : "size-10",
        className,
      )}
      {...rest}
    >
      {children}
    </button>
  );
});
