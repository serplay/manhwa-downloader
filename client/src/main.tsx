import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { Analytics } from "@vercel/analytics/react";
import { Toaster } from "sonner";
import { TooltipProvider } from "@/components/ui/Tooltip";
import App from "./App";
import "@/styles/theme.css";

const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 60_000,
      retry: (failureCount, error) => {
        // Upstream and validation failures will not fix themselves on retry.
        const status = (error as { status?: number }).status ?? 0;
        return status >= 500 || status === 0 ? failureCount < 1 : false;
      },
      refetchOnWindowFocus: false,
    },
  },
});

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <TooltipProvider delayDuration={300}>
        <App />
        <Toaster position="bottom-center" richColors={false} toastOptions={{ className: "font-sans" }} />
      </TooltipProvider>
    </QueryClientProvider>
    <Analytics />
  </StrictMode>,
);
