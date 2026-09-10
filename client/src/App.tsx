import { Header } from "@/components/layout/Header";

export default function App() {
  return (
    <div className="mx-auto flex min-h-dvh w-full max-w-6xl flex-col px-4 sm:px-6">
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-50 focus:rounded-control focus:bg-raised focus:px-3 focus:py-2 focus:text-sm"
      >
        Skip to content
      </a>
      <Header />
      <main id="main" className="flex-1 py-8">
        <p className="text-sm text-fg-muted">Search is on its way.</p>
      </main>
    </div>
  );
}
