import createClient, { type Middleware } from "openapi-fetch";
import type { paths } from "./schema";
import type { ErrorDetail } from "./types";

/** Prefix the API is reachable under: the Vite proxy in dev, the Vercel rewrite in prod. */
export const API_BASE = import.meta.env.VITE_API_BASE ?? "/api";

/** A failed request with the server's error envelope attached when it sent one. */
export class ApiError extends Error {
  readonly status: number;
  readonly code: string;
  readonly source: string | null;

  constructor(status: number, detail?: Partial<ErrorDetail> | null) {
    super(detail?.message ?? `Request failed with HTTP ${status}`);
    this.name = "ApiError";
    this.status = status;
    this.code = detail?.code ?? (status === 0 ? "NETWORK" : "HTTP_ERROR");
    this.source = detail?.source ?? null;
  }

  /** Human copy for the most common codes; falls back to the server message. */
  get friendly(): string {
    switch (this.code) {
      case "SOURCE_BLOCKED":
        return `${this.source ?? "This source"} needs a real browser to get past its Cloudflare check. Pick another source.`;
      case "UPSTREAM_TIMEOUT":
        return `${this.source ?? "The source"} did not respond in time. Try again or pick another source.`;
      case "NETWORK":
        return "Could not reach the server. Check your connection and try again.";
      default:
        return this.message;
    }
  }
}

const throwOnError: Middleware = {
  async onResponse({ response }) {
    if (response.ok) return response;
    let detail: Partial<ErrorDetail> | null = null;
    try {
      const body = (await response.clone().json()) as { error?: ErrorDetail };
      detail = body.error ?? null;
    } catch {
      detail = null;
    }
    throw new ApiError(response.status, detail);
  },
  async onError() {
    throw new ApiError(0, null);
  },
};

export const api = createClient<paths>({ baseUrl: API_BASE });
api.use(throwOnError);

/** Absolute-ish URL for a cover, routed through the server's image proxy. */
export function coverUrl(cover: { url: string; referer?: string | null } | null | undefined): string | null {
  if (!cover) return null;
  const params = new URLSearchParams({ url: cover.url });
  if (cover.referer) params.set("referer", cover.referer);
  return `${API_BASE}/proxy-image?${params.toString()}`;
}

export function fileUrl(taskId: string): string {
  return `${API_BASE}/download/file/${taskId}`;
}

export function eventsUrl(taskId: string): string {
  return `${API_BASE}/download/events/${taskId}`;
}
