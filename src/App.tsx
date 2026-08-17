import { BlurControl } from "@/components/BlurControl";
import { CyberBackground } from "@/components/CyberBackground";
import { OperationModal } from "@/components/OperationModal";
import { StatusIndicator } from "@/components/StatusIndicator";
import { useBlurMachine } from "@/hooks/useBlurMachine";

export default function App() {
  const { state, items, mode, modalOpen, overall, completed, closing, activate } =
    useBlurMachine();

  const running = state === "running" || state === "enabling";

  return (
    <main
      className={`theme-transition ${running ? "theme-green" : "theme-blue"} relative flex h-screen min-h-[560px] w-screen min-w-[780px] select-none flex-col overflow-hidden`}
      style={{ background: "var(--void)" }}
    >
      <CyberBackground />

      {/* title bar */}
      <header className="relative z-10 flex items-center justify-between px-7 pt-6">
        <div className="flex items-baseline gap-3">
          <span className="font-display text-[15px] font-bold tracking-[0.5em] text-white/90 text-glow">
            BLUR
          </span>
          <span className="font-mono-cyber text-[9.5px] tracking-[0.3em] text-white/30">
            NETWORK CONTROL v1.0
          </span>
        </div>
        <span className="font-mono-cyber text-[9.5px] tracking-[0.3em] text-white/30">
          {items.length || 3} ADAPTERS DETECTED
        </span>
      </header>

      {/* hero control */}
      <section className="relative z-10 flex flex-1 flex-col items-center justify-center gap-8">
        <BlurControl state={state} onActivate={activate} />
        <StatusIndicator state={state} />
      </section>

      <footer className="relative z-10 flex items-center justify-between px-7 pb-6 font-mono-cyber text-[9.5px] tracking-[0.28em] text-white/25">
        <span>{running ? "ADAPTERS OFFLINE" : "ADAPTERS ONLINE"}</span>
        <span>LOCAL SESSION · NO TELEMETRY</span>
      </footer>

      {modalOpen && (
        <OperationModal
          mode={mode}
          items={items}
          overall={overall}
          completed={completed}
          closing={closing}
        />
      )}
    </main>
  );
}
