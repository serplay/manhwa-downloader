import { Toaster } from "sonner";
import { useTheme } from "@/hooks/useTheme";

export function ThemedToaster() {
  const { resolved } = useTheme();
  return (
    <Toaster
      position="bottom-center"
      theme={resolved}
      toastOptions={{
        className: "font-sans rounded-control! bg-raised! text-fg! border-line! shadow-float!",
      }}
    />
  );
}
