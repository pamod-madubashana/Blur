import { BlurControl } from "@/components/BlurControl";
import { CyberBackground } from "@/components/CyberBackground";
import { LaunchModal } from "@/components/LaunchModal";
import { OperationModal } from "@/components/OperationModal";
import { FileCheckModal } from "@/components/FileCheckModal";
import { FirewallCheckModal } from "@/components/FirewallCheckModal";
import { StatusPanel } from "@/components/StatusPanel";
import { UpdateIndicator } from "@/components/UpdateIndicator";
import { useBlurMachine } from "@/hooks/useBlurMachine";
import { invoke } from "@tauri-apps/api/core";
import { getVersion } from "@tauri-apps/api/app";
import { useEffect, useState } from "react";

export default function App() {
  const { state, items, mode, adapterModalOpen, launchModalOpen, fileCheckModalOpen, checkPhase, fileItems, fileCheckDone, firewallItems, firewallCheckDone, discoveringItems, discoveringCheckDone, stepResults, overall, completed, closing, activate } =
    useBlurMachine();
  const [version, setVersion] = useState("");

  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
  }, []);

  const running = state === "running" || state === "enabling";
  const rulesModalOpen = state === "checking" && (checkPhase === "firewall" || checkPhase === "discovering");

  const ruleItems = checkPhase === "discovering" ? discoveringItems : firewallItems;
  const rulesDone = checkPhase === "discovering" ? discoveringCheckDone : firewallCheckDone;

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
        <div className="flex items-center gap-2">
          <button
            onClick={() => invoke("close_window")}
            className="flex h-6 w-6 items-center justify-center rounded transition-colors hover:bg-white/10"
            title="Close"
          >
            <svg width="10" height="10" viewBox="0 0 10 10" fill="none">
              <path d="M1 1L9 9M9 1L1 9" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" className="text-white/50" />
            </svg>
          </button>
        </div>
      </header>

      {/* circle */}
      <section className="relative z-10 flex flex-1 items-center justify-center">
        <BlurControl state={state} onActivate={activate} />
      </section>

      {/* update indicator */}
      <UpdateIndicator />

      {/* version */}
      <footer className="relative z-10 flex justify-end px-7 pb-5 font-mono-cyber text-[9.5px] tracking-[0.28em] text-white/25">
        <span>v{version}</span>
      </footer>

      {checkPhase === "file" && (
        <FileCheckModal items={fileItems} done={fileCheckDone} closing={closing} />
      )}

      {rulesModalOpen && (
        <FirewallCheckModal
          title={checkPhase === "discovering" ? "SHARING" : "FIREWALL"}
          items={ruleItems}
          done={rulesDone}
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
