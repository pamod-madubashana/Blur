import type { AppState } from "@/types/adapter";

const LABEL: Record<AppState, string> = {
  ready: "STANDBY · NETWORK ONLINE",
  preparing: "PREPARING SYSTEM...",
  disabling: "ISOLATING NETWORK",
  launching: "LAUNCHING GAME...",
  running: "GAME RUNNING · NETWORK ISOLATED",
  enabling: "RESTORING NETWORK",
};

export function StatusIndicator({ state }: { state: AppState }) {
  const busy = state === "disabling" || state === "enabling";
  return (
    <div
      className="inline-flex items-center gap-2.5 rounded-full px-4 py-1.5 font-mono-cyber text-[10.5px] tracking-[0.28em]"
      style={{
        border: "1px solid color-mix(in oklab, var(--accent) 30%, transparent)",
        background: "color-mix(in oklab, var(--accent) 8%, transparent)",
        color: "color-mix(in oklab, var(--accent-bright) 92%, white)",
      }}
    >
      <span
        className="h-1.5 w-1.5 rounded-full"
        style={{
          background: "var(--accent-bright)",
          boxShadow: "0 0 10px var(--accent-bright)",
          animation: `blur-breathe ${busy ? "0.8s" : "2.4s"} ease-in-out infinite`,
        }}
      />
      {LABEL[state]}
    </div>
  );
}
