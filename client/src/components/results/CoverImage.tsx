import { useState } from "react";
import { ImageBroken } from "@phosphor-icons/react";
import { coverUrl } from "@/api/client";
import type { Cover } from "@/api/types";
import { cn } from "@/lib/cn";

/** 2:3 cover through the image proxy with a shimmer while loading. */
export function CoverImage({
  cover,
  alt,
  className,
  priority = false,
}: {
  cover: Cover | null | undefined;
  alt: string;
  className?: string;
  /** Above the fold: load eagerly with high priority. */
  priority?: boolean;
}) {
  const [state, setState] = useState<"loading" | "loaded" | "error">("loading");
  const src = coverUrl(cover);
  const failed = !src || state === "error";
  return (
    <div className={cn("relative aspect-[2/3] w-full overflow-hidden rounded-control bg-surface", className)}>
      {!failed && state === "loading" && <div className="shimmer absolute inset-0" aria-hidden />}
      {failed ? (
        <div className="flex h-full items-center justify-center text-fg-faint">
          <ImageBroken size={24} />
        </div>
      ) : (
        <img
          src={src}
          alt={alt}
          loading={priority ? "eager" : "lazy"}
          fetchPriority={priority ? "high" : "auto"}
          decoding="async"
          onLoad={() => setState("loaded")}
          onError={() => setState("error")}
          className={cn(
            "h-full w-full object-cover transition-opacity duration-300",
            state === "loaded" ? "opacity-100" : "opacity-0",
          )}
        />
      )}
    </div>
  );
}
