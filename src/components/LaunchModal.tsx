interface Props {
  closing: boolean;
}

export function LaunchModal({ closing }: Props) {
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
      aria-label="Launching game"
    >
      <div
        className="cyber-panel relative w-full max-w-[340px] rounded-xl px-5 pb-6 pt-5"
        style={{ animation: `${closing ? "modal-out" : "modal-in"} 300ms cubic-bezier(0.16,1,0.3,1) both` }}
      >
        <span
          className="absolute inset-x-6 top-0 h-px"
          style={{ background: "linear-gradient(90deg, transparent, var(--accent-bright), transparent)" }}
        />

        <header className="mb-5 flex items-baseline justify-between">
          <h2 className="font-display text-[13px] font-semibold tracking-[0.3em] text-white/90 text-glow">
            GAME LAUNCH
          </h2>
        </header>

        <p className="mb-6 font-mono-cyber text-[10.5px] tracking-[0.18em] text-white/45">
          STARTING BLUR
        </p>

        <div className="flex flex-col items-center gap-5">
          {/* spinning ring */}
          <div className="relative h-16 w-16">
            <div
              className="absolute inset-0 rounded-full"
              style={{
                border: "2px solid color-mix(in oklab, var(--accent) 20%, transparent)",
              }}
            />
            <div
              className="absolute inset-0 rounded-full"
              style={{
                border: "2px solid transparent",
                borderTopColor: "var(--accent-bright)",
                borderRightColor: "var(--accent-bright)",
                animation: "blur-spin 1s linear infinite",
                boxShadow: "0 0 18px color-mix(in oklab, var(--accent) 40%, transparent)",
              }}
            />
            {/* center dot */}
            <div className="absolute inset-0 grid place-items-center">
              <div
                className="h-2 w-2 rounded-full"
                style={{
                  background: "var(--accent-bright)",
                  boxShadow: "0 0 12px var(--accent-bright)",
                  animation: "blur-pulse 1.5s ease-in-out infinite",
                }}
              />
            </div>
          </div>

          <div className="flex flex-col items-center gap-1.5">
            <span className="font-display text-[12px] font-semibold tracking-[0.3em] text-white/80 text-glow">
              WAITING FOR GAME
            </span>
            <span className="font-mono-cyber text-[9.5px] tracking-[0.2em] text-white/35">
              NETWORK ISOLATED · ADAPTERS OFFLINE
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
