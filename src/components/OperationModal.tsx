import { AdapterList } from "./AdapterList";
import type { AdapterProgress } from "@/types/adapter";

interface Props {
  mode: "disable" | "enable";
  items: AdapterProgress[];
  overall: number;
  completed: boolean;
  closing: boolean;
}

export function OperationModal({ mode, items, overall, completed, closing }: Props) {
  const title = mode === "disable" ? "NETWORK ISOLATION" : "NETWORK RESTORATION";
  const subtitle =
    mode === "disable" ? "Disabling virtual adapters" : "Re-enabling virtual adapters";
  const doneText = mode === "disable" ? "NETWORK ISOLATED" : "NETWORK RESTORED";

  const totalVirtual = items.filter((i) => i.adapter.type === "virtual").length;
  const doneVirtual = items.filter((i) => i.adapter.type === "virtual" && i.phase === "done").length;

  return (
    <div
      className="absolute inset-0 z-50 grid place-items-center"
      style={{
        background: "color-mix(in oklab, black 55%, transparent)",
        backdropFilter: "blur(7px) saturate(120%)",
        animation: `${closing ? "modal-out" : "blur-in"} 300ms ease both`,
      }}
      role="dialog"
      aria-modal="true"
      aria-label={title}
    >
      <div
        className="cyber-panel relative w-[420px] rounded-xl px-6 pb-5 pt-5"
        style={{ animation: `${closing ? "modal-out" : "modal-in"} 300ms cubic-bezier(0.16,1,0.3,1) both` }}
      >
        {/* top hairline */}
        <span
          className="absolute inset-x-6 top-0 h-px"
          style={{ background: "linear-gradient(90deg, transparent, var(--accent-bright), transparent)" }}
        />

        <header className="mb-4 flex items-baseline justify-between">
          <h2 className="font-display text-[13px] font-semibold tracking-[0.3em] text-white/90 text-glow">
            {title}
          </h2>
        </header>

        <p className="mb-4 font-mono-cyber text-[10.5px] tracking-[0.18em] text-white/45">
          {completed ? "OPERATION COMPLETE" : subtitle.toUpperCase()}
        </p>

        <AdapterList items={items} mode={mode} />

        <div className="mt-5">
          <div className="mb-2 flex items-center justify-between font-mono-cyber text-[9.5px] tracking-[0.28em] text-white/40">
            <span>SYSTEM OPERATION</span>
            <span>{doneVirtual}/{totalVirtual}</span>
          </div>
          <div
            className="h-1.5 overflow-hidden rounded-full"
            style={{ background: "color-mix(in oklab, var(--accent) 12%, transparent)" }}
          >
            <div
              className="h-full rounded-full"
              style={{
                width: `${overall}%`,
                background: "linear-gradient(90deg, var(--accent-deep), var(--accent-bright))",
                boxShadow: "0 0 14px var(--accent)",
                transition: "width 260ms linear",
              }}
            />
          </div>
        </div>

        {completed && (
          <div
            className="mt-4 flex items-center justify-center gap-2 rounded-md py-2.5"
            style={{
              border: "1px solid color-mix(in oklab, var(--accent) 40%, transparent)",
              background: "color-mix(in oklab, var(--accent) 12%, transparent)",
              animation: "blur-in 320ms cubic-bezier(0.16,1,0.3,1) both",
            }}
          >
            <span
              className="h-1.5 w-1.5 rounded-full"
              style={{ background: "var(--accent-bright)", boxShadow: "0 0 10px var(--accent-bright)" }}
            />
            <span className="font-display text-[12px] font-semibold tracking-[0.34em] text-white/90 text-glow">
              {doneText}
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
