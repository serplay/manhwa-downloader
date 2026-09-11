import { Popover } from "radix-ui";
import { GearSix, Monitor, Moon, Sun } from "@phosphor-icons/react";
import { IconButton } from "@/components/ui/IconButton";
import { Segmented } from "@/components/ui/Segmented";
import { Switch } from "@/components/ui/Switch";
import { usePrefs } from "@/store/prefs";
import { SourceStatusPopover } from "@/components/sources/SourceStatusPopover";
import { useTheme } from "@/hooks/useTheme";
import type { ThemeMode } from "@/lib/theme";
import type { ReactNode } from "react";

const themeOptions: { value: ThemeMode; label: ReactNode; title: string }[] = [
  { value: "system", label: <Monitor size={16} />, title: "Follow system" },
  { value: "light", label: <Sun size={16} />, title: "Light" },
  { value: "dark", label: <Moon size={16} />, title: "Dark" },
];

export function Header({ actions }: { actions?: ReactNode }) {
  const { mode, setMode } = useTheme();
  const { showAdult, setShowAdult } = usePrefs();
  return (
    <header className="flex h-16 items-center justify-between gap-4">
      <a href="/" className="flex items-center gap-3 rounded-control">
        <img
          src="/logo-192.png"
          width={36}
          height={36}
          alt=""
          className="size-9 rounded-[10px]"
          decoding="async"
        />
        <span className="text-lg font-semibold tracking-tight">Manhwa Downloader</span>
      </a>
      <div className="flex items-center gap-1">
        {actions}
        <SourceStatusPopover />
        <Popover.Root>
          <Popover.Trigger asChild>
            <IconButton label="Settings">
              <GearSix size={18} />
            </IconButton>
          </Popover.Trigger>
          <Popover.Portal>
            <Popover.Content
              align="end"
              sideOffset={8}
              className="z-30 w-64 rounded-panel bg-raised p-4 shadow-float hairline"
            >
              <p className="mb-2 text-xs font-medium text-fg-muted">Appearance</p>
              <Segmented
                label="Theme"
                value={mode}
                onChange={setMode}
                options={themeOptions}
                className="w-full [&>button]:flex-1"
              />
              <div className="my-4 border-t border-line" />
              <p className="mb-2 text-xs font-medium text-fg-muted">Content</p>
              <Switch
                checked={showAdult}
                onCheckedChange={setShowAdult}
                label="Show adult content"
                description="Includes 18+ titles and adult-oriented sources in search."
              />
            </Popover.Content>
          </Popover.Portal>
        </Popover.Root>
      </div>
    </header>
  );
}
