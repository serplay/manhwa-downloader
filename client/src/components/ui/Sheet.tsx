import type { ReactNode } from "react";
import { Dialog } from "radix-ui";
import { X } from "@phosphor-icons/react";
import { IconButton } from "./IconButton";
import { cn } from "@/lib/cn";

/**
 * Right slide-over on `md+`, bottom sheet below. Radix Dialog handles focus
 * trap, escape, and scroll lock; the enter/exit motion is CSS keyed on
 * `data-state` so it collapses under reduced motion.
 */
export function Sheet({
  open,
  onOpenChange,
  title,
  description,
  children,
  className,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  title: ReactNode;
  description?: ReactNode;
  children: ReactNode;
  className?: string;
}) {
  return (
    <Dialog.Root open={open} onOpenChange={onOpenChange}>
      <Dialog.Portal>
        <Dialog.Overlay className="sheet-overlay fixed inset-0 z-40 bg-fg/30 backdrop-blur-[2px] dark:bg-black/50" />
        <Dialog.Content
          className={cn(
            "sheet-panel fixed z-50 flex flex-col bg-bg shadow-float outline-none",
            "inset-x-0 bottom-0 max-h-[92dvh] rounded-t-panel",
            "md:inset-y-0 md:right-0 md:left-auto md:h-dvh md:max-h-none md:w-[min(40rem,92vw)] md:rounded-l-panel md:rounded-tr-none",
            className,
          )}
        >
          <div className="flex items-start justify-between gap-4 px-5 pt-5 pb-3 md:px-6">
            <div className="min-w-0">
              <Dialog.Title className="text-base font-semibold tracking-tight">{title}</Dialog.Title>
              {description ? (
                <Dialog.Description className="mt-0.5 text-xs text-fg-muted">{description}</Dialog.Description>
              ) : (
                <Dialog.Description className="sr-only">Dialog</Dialog.Description>
              )}
            </div>
            <Dialog.Close asChild>
              <IconButton label="Close" size="sm" className="-mt-1 -mr-1">
                <X size={16} />
              </IconButton>
            </Dialog.Close>
          </div>
          {children}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
