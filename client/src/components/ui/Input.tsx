import { forwardRef, useId, type InputHTMLAttributes, type ReactNode } from "react";
import { cn } from "@/lib/cn";

export interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  label: string;
  /** Visually hide the label (it stays in the accessibility tree). */
  hideLabel?: boolean;
  hint?: string;
  error?: string;
  leading?: ReactNode;
  trailing?: ReactNode;
  wrapperClassName?: string;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { id, label, hideLabel, hint, error, leading, trailing, className, wrapperClassName, ...rest },
  ref,
) {
  const autoId = useId();
  const inputId = id ?? autoId;
  const hintId = hint ? `${inputId}-hint` : undefined;
  const errorId = error ? `${inputId}-error` : undefined;
  return (
    <div className={cn("flex flex-col gap-1.5", wrapperClassName)}>
      <label htmlFor={inputId} className={cn("text-xs font-medium text-fg-muted", hideLabel && "sr-only")}>
        {label}
      </label>
      <div
        className={cn(
          "flex h-10 items-center gap-2 rounded-control bg-raised px-3 hairline",
          "transition-shadow focus-within:shadow-[inset_0_0_0_1.5px_var(--color-accent)]",
          error && "shadow-[inset_0_0_0_1.5px_var(--color-danger)]",
        )}
      >
        {leading && <span className="shrink-0 text-fg-faint">{leading}</span>}
        <input
          ref={ref}
          id={inputId}
          aria-invalid={error ? true : undefined}
          aria-describedby={[hintId, errorId].filter(Boolean).join(" ") || undefined}
          className={cn(
            "min-w-0 flex-1 bg-transparent text-sm text-fg placeholder:text-fg-faint outline-none",
            className,
          )}
          {...rest}
        />
        {trailing && <span className="shrink-0">{trailing}</span>}
      </div>
      {error ? (
        <p id={errorId} className="text-xs text-danger">
          {error}
        </p>
      ) : hint ? (
        <p id={hintId} className="text-xs text-fg-faint">
          {hint}
        </p>
      ) : null}
    </div>
  );
});
