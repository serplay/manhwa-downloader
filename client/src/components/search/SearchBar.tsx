import { useEffect, useState, type FormEvent } from "react";
import { MagnifyingGlass } from "@phosphor-icons/react";
import { Button } from "@/components/ui/Button";
import { Input } from "@/components/ui/Input";
import { Select, type SelectOption } from "@/components/ui/Select";
import { StatusDot } from "@/components/sources/StatusDot";
import { useRecentSearches } from "@/hooks/useRecentSearches";
import { ALL_SOURCES, isUsable, speedLabel, useSources } from "@/hooks/useSources";
import { RecentSearches } from "./RecentSearches";

export function SearchBar({
  query,
  source,
  onSearch,
  busy,
}: {
  query: string;
  source: string;
  onSearch: (q: string, source: string) => void;
  busy?: boolean;
}) {
  const [text, setText] = useState(query);
  const [picked, setPicked] = useState(source || ALL_SOURCES);
  const sources = useSources();
  const recent = useRecentSearches();
  const [focused, setFocused] = useState(false);

  // Keep local fields in sync with URL-driven changes (back button, chips).
  useEffect(() => setText(query), [query]);
  useEffect(() => setPicked(source || ALL_SOURCES), [source]);

  const options: SelectOption<string>[] = [
    { value: ALL_SOURCES, label: "All sources", description: "Every source that is up" },
    ...(sources.data ?? []).map((s) => ({
      value: s.slug,
      disabled: !isUsable(s),
      triggerLabel: s.name,
      label: (
        <span className="flex items-center gap-2">
          <StatusDot status={s.status} />
          {s.name}
        </span>
      ),
      description: isUsable(s) ? speedLabel(s.speed) : "Needs a browser, not available yet",
    })),
  ];

  const run = (q: string, s: string) => {
    recent.add(q);
    onSearch(q, s);
  };
  const submit = (e: FormEvent) => {
    e.preventDefault();
    const q = text.trim();
    if (q) run(q, picked);
  };
  const showRecent = (focused && text.trim() === "") || (!query && text.trim() === "");

  return (
    <div className="flex flex-col gap-3">
    <form onSubmit={submit} className="grid grid-cols-1 gap-3 sm:grid-cols-[13rem_1fr_auto] sm:items-end">
      <div className="flex flex-col gap-1.5">
        <span className="text-xs font-medium text-fg-muted">Source</span>
        <Select label="Source" value={picked} onChange={setPicked} options={options} />
      </div>
      <Input
        label="Search titles"
        name="q"
        type="search"
        autoComplete="off"
        enterKeyHint="search"
        placeholder="Solo Leveling"
        value={text}
        onChange={(e) => setText(e.target.value)}
        onFocus={() => setFocused(true)}
        onBlur={() => setFocused(false)}
        leading={<MagnifyingGlass size={16} />}
      />
      <Button type="submit" variant="primary" size="md" disabled={busy || !text.trim()} className="sm:h-10">
        Search
      </Button>
    </form>
      {showRecent && <RecentSearches onPick={(q) => run(q, picked)} />}
    </div>
  );
}
