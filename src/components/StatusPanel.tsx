import type { RunStep } from "@/hooks/useBlurMachine";

interface Props {
  steps: RunStep[];
}

const STEP_LABELS: Record<string, string> = {
  files: "Online Fix Files",
  firewall: "Firewall Rules",
  discovering: "Discovery Services",
  adapters: "Network Adapters",
  launch: "Game Launch",
};

const STEP_ORDER = ["files", "firewall", "discovering", "adapters", "launch"];

export function StatusPanel({ steps }: Props) {
  const getStep = (key: string) => steps.find((s) => s.step === key);

  return (
    <div
      className="absolute bottom-5 left-5 z-40"
      style={{ animation: "blur-in 400ms cubic-bezier(0.16,1,0.3,1) both" }}
    >
      <div
        className="cyber-panel relative w-[220px] rounded-lg px-4 py-3"
        style={{
          background: "color-mix(in oklab, var(--void) 85%, black)",
          border: "1px solid color-mix(in oklab, var(--accent) 20%, transparent)",
          backdropFilter: "blur(8px)",
        }}
      >
        <span
          className="absolute inset-x-4 top-0 h-px"
          style={{ background: "linear-gradient(90deg, transparent, var(--accent-bright), transparent)" }}
        />

        <h3 className="mb-2.5 font-display text-[10px] font-semibold tracking-[0.35em] text-white/60">
          STATUS
        </h3>

        <div className="flex flex-col gap-1.5">
          {STEP_ORDER.map((key) => {
            const step = getStep(key);
            const status = step?.status;
            const isFailed = status === "failed";
            const isOk = status === "ok";
            const isPending = !status;

            return (
              <div key={key} className="flex items-center gap-2">
                <span
                  className="flex h-4 w-4 shrink-0 items-center justify-center rounded-sm"
                  style={{
                    background: isFailed
                      ? "rgba(239, 68, 68, 0.15)"
                      : isOk
                        ? "color-mix(in oklab, var(--accent) 12%, transparent)"
                        : "rgba(255, 255, 255, 0.04)",
                    border: `1px solid ${isFailed ? "rgba(239, 68, 68, 0.3)" : isOk ? "color-mix(in oklab, var(--accent) 25%, transparent)" : "rgba(255, 255, 255, 0.06)"}`,
                  }}
                >
                  {isFailed && (
                    <svg width="8" height="8" viewBox="0 0 8 8" fill="none">
                      <path d="M1 1L7 7M7 1L1 7" stroke="#ef4444" strokeWidth="1.5" strokeLinecap="round" />
                    </svg>
                  )}
                  {isOk && (
                    <svg width="8" height="8" viewBox="0 0 8 8" fill="none">
                      <path d="M1 4L3 6L7 2" stroke="var(--accent-bright)" strokeWidth="1.5" strokeLinecap="round" strokeLinejoin="round" />
                    </svg>
                  )}
                  {isPending && (
                    <span
                      className="h-1.5 w-1.5 rounded-full"
                      style={{ background: "rgba(255,255,255,0.15)" }}
                    />
                  )}
                </span>

                <span
                  className="font-mono-cyber text-[9px] tracking-[0.14em]"
                  style={{
                    color: isFailed
                      ? "#ef4444"
                      : isOk
                        ? "color-mix(in oklab, var(--accent-bright) 80%, white)"
                        : "rgba(255,255,255,0.25)",
                  }}
                >
                  {STEP_LABELS[key]}
                </span>
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
}
