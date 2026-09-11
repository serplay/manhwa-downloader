import type { Format } from "@/api/types";
import { Segmented } from "@/components/ui/Segmented";

const LABEL: Record<Format, string> = { pdf: "PDF", cbz: "CBZ", epub: "EPUB", cbr: "CBR" };

export function FormatPicker({ value, onChange, formats }: { value: Format; onChange: (f: Format) => void; formats: Format[] }) {
  return (
    <Segmented
      label="Format"
      size="sm"
      value={value}
      onChange={onChange}
      options={formats.map((f) => ({ value: f, label: LABEL[f] }))}
    />
  );
}
