import type { FileCheckItem, FirewallCheckItem, DiscoveringCheckItem } from "@/hooks/useBlurMachine";

interface Props {
  fileItems: FileCheckItem[];
  fileDone: boolean;
  firewallItems: FirewallCheckItem[];
  firewallDone: boolean;
  discoveringItems: DiscoveringCheckItem[];
  discoveringDone: boolean;
  closing: boolean;
}

function SectionProgress({ total, ok }: { total: number; ok: number }) {
  const pct = total > 0 ? (ok / total) * 100 : 0;
  return (
    <div className="mt-2">
      <div className="mb-1 flex items-center justify-between font-mono-cyber text-[8.5px] tracking-[0.24em] text-white/35">
        <span>{ok}/{total}</span>
      </div>
      <div
        className="h-1 overflow-hidden rounded-full"
        style={{ background: "color-mix(in oklab, var(--accent) 10%, transparent)" }}
      >
        <div
          className="h-full rounded-full"
          style={{
            width: `${pct}%`,
            background: "linear-gradient(90deg, var(--accent-deep), var(--accent-bright))",
            boxShadow: "0 0 10px var(--accent)",
            transition: "width 260ms linear",
          }}
        />
      </div>
    </div>
  );
}

export function PreparingModal({
  fileItems,
  fileDone,
  firewallItems,
  firewallDone,
  discoveringItems,
  discoveringDone,
  closing,
}: Props) {
  const allDone = fileDone && firewallDone && discoveringDone;

  const fileOk = fileItems.filter((i) => i.status === "ok" || i.status === "copied").length;
  const fwOk = firewallItems.filter((i) => i.status === "ok" || i.status === "created").length;
  const discOk = discoveringItems.filter((i) => i.status === "ok" || i.status === "enabled" || i.status === "started").length;

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
      aria-label="System preparation"
    >
      <div
        className="cyber-panel relative w-full max-w-[380px] rounded-xl px-5 pb-5 pt-5"
        style={{ animation: `${closing ? "modal-out" : "modal-in"} 300ms cubic-bezier(0.16,1,0.3,1) both` }}
      >
        <span
          className="absolute inset-x-6 top-0 h-px"
          style={{ background: "linear-gradient(90deg, transparent, var(--accent-bright), transparent)" }}
        />

        <header className="mb-4">
          <h2 className="font-display text-[13px] font-semibold tracking-[0.3em] text-white/90 text-glow">
            SYSTEM PREPARATION
          </h2>
          <p className="mt-1 font-mono-cyber text-[10px] tracking-[0.18em] text-white/40">
            {allDone ? "ALL CHECKS PASSED" : "CHECKING FILES & RULES..."}
          </p>
        </header>

        <div className="flex flex-col gap-3">
          {/* ONLINE FIX section */}
          <Section
            title="ONLINE FIX"
            delay={0}
            done={fileDone}
            items={fileItems.map((i) => ({
              label: i.file,
              status: i.status === "ok" ? ("ok" as const) : i.status === "copied" ? ("created" as const) : ("checking" as const),
            }))}
            okCount={fileOk}
            closing={closing}
          />

          {/* FIREWALL section */}
          <Section
            title="FIREWALL"
            delay={80}
            done={firewallDone}
            items={firewallItems.map((i) => ({
              label: i.rule,
              status: i.status === "failed" ? ("failed" as const) : i.status === "ok" || i.status === "created" ? ("ok" as const) : ("checking" as const),
            }))}
            okCount={fwOk}
            closing={closing}
          />

          {/* SHARING section */}
          <Section
            title="SHARING"
            delay={160}
            done={discoveringDone}
            items={discoveringItems.map((i) => ({
              label: i.rule,
              status: i.status === "failed" ? ("failed" as const) : i.status === "ok" || i.status === "enabled" || i.status === "started" ? ("ok" as const) : ("checking" as const),
            }))}
            okCount={discOk}
            closing={closing}
          />
        </div>

        {allDone && (
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
              PREPARATION OK
            </span>
          </div>
        )}
      </div>
    </div>
  );
}

/* ── internal Section component ────────────────────────────── */

interface SectionItem {
  label: string;
  status: "ok" | "created" | "failed" | "checking";
}

interface SectionProps {
  title: string;
  delay: number;
  done: boolean;
  items: SectionItem[];
  okCount: number;
  closing: boolean;
}

function Section({ title, delay, done, items, okCount, closing }: SectionProps) {
  const total = items.length;
  const failedCount = items.filter((i) => i.status === "failed").length;

  return (
    <div
      className="rounded-lg px-3.5 py-3"
      style={{
        background: "color-mix(in oklab, var(--void) 60%, transparent)",
        border: "1px solid color-mix(in oklab, var(--accent) 12%, transparent)",
        animation: `${closing ? "modal-out" : "row-in"} 300ms cubic-bezier(0.16,1,0.3,1) ${delay}ms both`,
      }}
    >
      <div className="mb-1.5 flex items-center justify-between">
        <span className="font-display text-[10px] font-semibold tracking-[0.3em] text-white/70">
          {title}
        </span>
        <span
          className="font-mono-cyber text-[9px] tracking-[0.16em]"
          style={{
            color: done
              ? failedCount > 0
                ? "#ef4444"
                : "color-mix(in oklab, var(--accent-bright) 80%, white)"
              : "white/30",
          }}
        >
          {done ? (failedCount > 0 ? "PARTIAL" : "OK") : "..."}
        </span>
      </div>

      {total > 0 && (
        <div className="max-h-[80px] overflow-y-auto pr-1">
          {items.map((item, idx) => (
            <div
              key={`${item.label}-${idx}`}
              className="flex items-center justify-between border-b py-1"
              style={{ borderColor: "color-mix(in oklab, var(--accent) 6%, transparent)" }}
            >
              <span className="font-mono-cyber text-[8.5px] tracking-[0.1em] text-white/45 truncate max-[200px]">
                {item.label}
              </span>
              <span
                className="font-mono-cyber text-[8px] tracking-[0.14em]"
                style={{
                  color: item.status === "ok"
                    ? "color-mix(in oklab, var(--accent-bright) 80%, white)"
                    : item.status === "created"
                      ? "#f0c040"
                      : item.status === "failed"
                        ? "#ef4444"
                        : "white/30",
                }}
              >
                {item.status === "ok" ? "OK" : item.status === "created" ? "FIXED" : item.status === "failed" ? "FAIL" : "..."}
              </span>
            </div>
          ))}
        </div>
      )}

      <SectionProgress total={total} ok={okCount} />
    </div>
  );
}
