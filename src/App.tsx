import { BlurControl } from "@/components/BlurControl";
import { CyberBackground } from "@/components/CyberBackground";
import { LaunchModal } from "@/components/LaunchModal";
import { OperationModal } from "@/components/OperationModal";
import { FileCheckModal } from "@/components/FileCheckModal";
import { useBlurMachine } from "@/hooks/useBlurMachine";

export default function App() {
  const { state, items, mode, adapterModalOpen, launchModalOpen, fileCheckModalOpen, fileItems, fileCheckDone, overall, completed, closing, activate } =
    useBlurMachine();

  const running = state === "running" || state === "enabling";

  return (
    <main
      className={`theme-transition ${running ? "theme-green" : "theme-blue"} relative flex h-screen w-screen select-none flex-col overflow-hidden`}
      style={{ background: "var(--void)" }}
    >
      <CyberBackground />

      {/* title */}
      <header className="relative z-10 px-7 pt-6">
        <span className="font-display text-[15px] font-bold tracking-[0.5em] text-white/90 text-glow">
          BLUR
        </span>
      </header>

      {/* circle */}
      <section className="relative z-10 flex flex-1 items-center justify-center">
        <BlurControl state={state} onActivate={activate} />
      </section>

      {/* version */}
      <footer className="relative z-10 flex justify-end px-7 pb-5 font-mono-cyber text-[9.5px] tracking-[0.28em] text-white/25">
        <span>v1.0</span>
      </footer>

      {fileCheckModalOpen && (
        <FileCheckModal items={fileItems} done={fileCheckDone} closing={closing} />
      )}

      {adapterModalOpen && (
        <OperationModal
          mode={mode}
          items={items}
          overall={overall}
          completed={completed}
          closing={closing}
        />
      )}

      {launchModalOpen && (
        <LaunchModal closing={closing} />
      )}
    </main>
  );
}
