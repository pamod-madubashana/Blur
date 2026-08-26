import { BlurControl } from "@/components/BlurControl";
import { CyberBackground } from "@/components/CyberBackground";
import { LaunchModal } from "@/components/LaunchModal";
import { OperationModal } from "@/components/OperationModal";
import { PreparingModal } from "@/components/PreparingModal";
import { StatusPanel } from "@/components/StatusPanel";
import { UpdatePopup } from "@/components/UpdatePopup";
import { useBlurMachine } from "@/hooks/useBlurMachine";
import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";

export default function App() {
  const { state, items, mode, adapterModalOpen, launchModalOpen, preparingOpen, fileItems, fileCheckDone, firewallItems, firewallCheckDone, discoveringItems, discoveringCheckDone, stepResults, overall, completed, closing, activate, updateInfo, showUpdatePopup, dismissUpdate } =
    useBlurMachine();
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
  }, []);

  const running = state === "running" || state === "enabling";

  return (
    <main
      className={`theme-transition ${running ? "theme-green" : "theme-blue"} relative flex h-screen w-screen select-none flex-col overflow-hidden`}
      style={{ background: "var(--void)" }}
    >
      <CyberBackground />

      {/* title */}
      <header className="relative z-10 flex items-center justify-between px-7 pt-6">
        <span className="font-display text-[15px] font-bold tracking-[0.5em] text-white/90 text-glow">
          BLUR
        </span>
      </header>

      {/* circle */}
      <section className="relative z-10 flex flex-1 items-center justify-center">
        <BlurControl state={state} onActivate={activate} />
      </section>

      {/* update popup */}
      {showUpdatePopup && updateInfo && (
        <UpdatePopup updateInfo={updateInfo} onDismiss={dismissUpdate} />
      )}

      {/* footer */}
      <footer className="relative z-10 flex items-center justify-between px-7 pb-7">
        <a
          href="https://github.com/pamod-madubashana/Blur"
          target="_blank"
          rel="noopener noreferrer"
          className="font-mono-cyber text-[9.5px] tracking-[0.28em] text-white/25 transition-colors hover:text-white/50"
        >
          GitHub
        </a>
        <span className="font-mono-cyber text-[9.5px] tracking-[0.28em] text-white/25">
          v{version}
        </span>
      </footer>

      {preparingOpen && (
        <PreparingModal
          fileItems={fileItems}
          fileDone={fileCheckDone}
          firewallItems={firewallItems}
          firewallDone={firewallCheckDone}
          discoveringItems={discoveringItems}
          discoveringDone={discoveringCheckDone}
          closing={closing}
        />
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

      {running && stepResults.length > 0 && (
        <StatusPanel steps={stepResults} />
      )}
    </main>
  );
}
