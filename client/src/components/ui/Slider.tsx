import { Slider as RadixSlider } from "radix-ui";
import { cn } from "@/lib/cn";

/** Two-thumb range slider. Values are indices, labels come from the caller. */
export function RangeSlider({
  min,
  max,
  value,
  onChange,
  onCommit,
  label,
  className,
  thumbLabel,
}: {
  min: number;
  max: number;
  value: [number, number];
  onChange: (v: [number, number]) => void;
  onCommit?: (v: [number, number]) => void;
  label: string;
  className?: string;
  /** Text for aria-valuetext, given the raw index. */
  thumbLabel?: (v: number) => string;
}) {
  return (
    <RadixSlider.Root
      min={min}
      max={max}
      step={1}
      minStepsBetweenThumbs={0}
      value={value}
      onValueChange={(v) => onChange([v[0] ?? min, v[1] ?? max])}
      onValueCommit={(v) => onCommit?.([v[0] ?? min, v[1] ?? max])}
      aria-label={label}
      className={cn("relative flex h-6 w-full touch-none items-center select-none", className)}
    >
      <RadixSlider.Track className="relative h-1 grow rounded-full bg-line-strong">
        <RadixSlider.Range className="absolute h-full rounded-full bg-accent" />
      </RadixSlider.Track>
      {[0, 1].map((i) => (
        <RadixSlider.Thumb
          key={i}
          aria-label={i === 0 ? "From" : "To"}
          aria-valuetext={thumbLabel?.(value[i as 0 | 1])}
          className="block size-4 rounded-full bg-raised shadow-lift ring-2 ring-accent transition-transform hover:scale-110 focus-visible:outline-none focus-visible:ring-4 focus-visible:ring-accent/40"
        />
      ))}
    </RadixSlider.Root>
  );
}
