import { Check, Loader2 } from "lucide-react";
import type { AdapterProgress } from "@/types/adapter";

interface Props {
  item: AdapterProgress;
  index: number;
  mode: "disable" | "enable";
}

export function AdapterRow({ item, index, mode }: Props) {
  const { phase, adapter } = item;
  const doneWord = mode === "disable" ? "Disabled" : "Enabled";
  const activeWord = mode === "disable" ? "Disabling\u2026" : "Enabling\u2026";
  const statusText = phase === "done" ? doneWord : phase === "processing" ? activeWord : "Pending";

  return (
    <li
      className="relative flex items-center gap-3 rounded-md px-3 py-2.5"
      style={{
        animation: `row-in 340ms cubic-bezier(0.16,1,0.3,1) ${index * 70}ms both`,
        background:
          phase === "pending"
            ? "transparent"
            : "color-mix(in oklab, var(--accent) 7%, transparent)",
        border: `1px solid color-mix(in oklab, var(--accent) ${phase === "pending" ? 10 : 26}%, transparent)`,
        transition: "background 400ms ease, border-color 400ms ease, opacity 400ms ease",
        opacity: phase === "pending" ? 0.5 : 1,
      }}
    >
      <span className="grid h-5 w-5 place-items-center">
        {phase === "done" ? (
          <Check
            className="h-4 w-4"
            style={{ color: "var(--accent-bright)", animation: "blur-in 260ms ease-out" }}
          />
        ) : phase === "processing" ? (
          <Loader2 className="h-4 w-4 animate-spin" style={{ color: "var(--accent-bright)" }} />
        ) : (
          <span
            className="h-2 w-2 rounded-full"
            style={{ border: "1px solid color-mix(in oklab, var(--accent) 55%, transparent)" }}
          />
        )}
      </span>

      <span className="flex-1 font-display text-[13px] tracking-[0.08em] text-white/85">
        {adapter.name}
      </span>

      <span
        className="font-mono-cyber text-[10px] tracking-[0.2em]"
        style={{ color: "color-mix(in oklab, var(--accent-bright) 85%, white)", opacity: phase === "pending" ? 0.5 : 1 }}
      >
        {statusText}
      </span>
    </li>
  );
}
