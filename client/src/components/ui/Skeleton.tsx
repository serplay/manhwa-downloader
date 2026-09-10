import type { HTMLAttributes } from "react";
import { cn } from "@/lib/cn";

/** Placeholder shaped like the content it stands in for. */
export function Skeleton({ className, ...rest }: HTMLAttributes<HTMLDivElement>) {
  return <div aria-hidden className={cn("shimmer rounded-chip", className)} {...rest} />;
}
