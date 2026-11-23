import type { LanSnapshot } from "@/types/lan";

interface Props {
  lan: LanSnapshot;
}

export function LanPlayersPanel({ lan }: Props) {
  const peers = lan.peers.slice(0, 5);

  return (
    <div
      className="absolute right-5 top-[68px] z-40"
      style={{ animation: "blur-in 400ms cubic-bezier(0.16,1,0.3,1) both" }}
    >
      <div
        className="cyber-panel relative w-[164px] rounded-lg px-3.5 py-3"
        style={{
          background: "color-mix(in oklab, var(--void) 85%, black)",
          border: "1px solid color-mix(in oklab, var(--accent) 20%, transparent)",
          backdropFilter: "blur(8px)",
        }}
      >
        <span
          className="absolute inset-x-3 top-0 h-px"
          style={{ background: "linear-gradient(90deg, transparent, var(--accent-bright), transparent)" }}
        />

        <div className="mb-2.5 flex items-baseline justify-between">
          <h3 className="font-display text-[9px] font-semibold tracking-[0.3em] text-white/60">
            LAN PLAYERS
          </h3>
          <span
            className="font-mono-cyber text-[9px]"
            style={{ color: "var(--accent-bright)" }}
          >
            {lan.peers.length.toString().padStart(2, "0")}
          </span>
        </div>

        {!lan.listening ? (
          <p className="font-mono-cyber text-[8.5px] leading-relaxed tracking-[0.1em] text-white/35">
            {lan.error ? "LISTENER ERROR" : "PORT 50001 BUSY"}
          </p>
        ) : peers.length === 0 ? (
          <p className="font-mono-cyber text-[8.5px] leading-relaxed tracking-[0.1em] text-white/35">
            SCANNING LAN...
          </p>
        ) : (
          <div className="flex flex-col gap-2">
            {peers.map((peer) => (
              <div key={peer.address} className="flex flex-col gap-0.5">
                <div className="flex items-center gap-1.5">
                  <span
                    className="h-1.5 w-1.5 shrink-0 rounded-full"
                    style={{
                      background: "var(--accent-bright)",
                      boxShadow: "0 0 8px var(--accent-bright)",
                    }}
                  />
                  <span
                    className="truncate font-mono-cyber text-[9px] tracking-[0.08em]"
                    style={{ color: "color-mix(in oklab, var(--accent-bright) 80%, white)" }}
                    title={peer.strings.join(" | ")}
                  >
                    {peer.name}
                  </span>
                </div>
                <span className="pl-3 font-mono-cyber text-[8px] tracking-[0.08em] text-white/30">
                  {peer.address}:{peer.port} · {peer.last_seen}s
                </span>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}
