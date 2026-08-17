import type { FirewallCheckItem } from "@/hooks/useBlurMachine";

interface Props {
  items: FirewallCheckItem[];
  done: boolean;
  closing: boolean;
}

export function FirewallCheckModal({ items, done, closing }: Props) {
  const okCount = items.filter((i) => i.status === "ok" || i.status === "created").length;
  const total = items.length;

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
      aria-label="Firewall check"
    >
      <div
        className="cyber-panel relative w-full max-w-[340px] rounded-xl px-5 pb-5 pt-5"
        style={{ animation: `${closing ? "modal-out" : "modal-in"} 300ms cubic-bezier(0.16,1,0.3,1) both` }}
      >
        <span
          className="absolute inset-x-6 top-0 h-px"
          style={{ background: "linear-gradient(90deg, transparent, var(--accent-bright), transparent)" }}
        />

        <header className="mb-4 flex items-baseline justify-between">
          <h2 className="font-display text-[13px] font-semibold tracking-[0.3em] text-white/90 text-glow">
            FIREWALL
          </h2>
        </header>

        <p className="mb-4 font-mono-cyber text-[10.5px] tracking-[0.18em] text-white/45">
          {done ? "FIREWALL READY" : "CHECKING RULES..."}
        </p>

        <div className="max-h-[200px] overflow-y-auto pr-1" style={{ scrollbarWidth: "thin" }}>
          {items.map((item) => (
            <div
              key={item.rule}
              className="flex items-center justify-between border-b py-1.5"
              style={{ borderColor: "color-mix(in oklab, var(--accent) 10%, transparent)" }}
            >
              <span className="font-mono-cyber text-[9.5px] tracking-[0.12em] text-white/60 truncate max-w-[200px]">
                {item.rule}
              </span>
              <span
                className="font-mono-cyber text-[9px] tracking-[0.16em]"
                style={{
                  color: item.status === "ok"
                    ? "color-mix(in oklab, var(--accent-bright) 80%, white)"
                    : item.status === "created"
                      ? "#f0c040"
                      : "white/40",
                }}
              >
                {item.status === "ok" ? "OK" : item.status === "created" ? "CREATED" : "..."}
              </span>
            </div>
          ))}
        </div>

        {total > 0 && (
          <div className="mt-4">
            <div className="mb-2 flex items-center justify-between font-mono-cyber text-[9.5px] tracking-[0.28em] text-white/40">
              <span>RULES CHECKED</span>
              <span>{okCount}/{total}</span>
            </div>
            <div
              className="h-1.5 overflow-hidden rounded-full"
              style={{ background: "color-mix(in oklab, var(--accent) 12%, transparent)" }}
            >
              <div
                className="h-full rounded-full"
                style={{
                  width: `${total > 0 ? (okCount / total) * 100 : 0}%`,
                  background: "linear-gradient(90deg, var(--accent-deep), var(--accent-bright))",
                  boxShadow: "0 0 14px var(--accent)",
                  transition: "width 260ms linear",
                }}
              />
            </div>
          </div>
        )}

        {done && (
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
              FIREWALL OK
            </span>
          </div>
        )}
      </div>
    </div>
  );
}
