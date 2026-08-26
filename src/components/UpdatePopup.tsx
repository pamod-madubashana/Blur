import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useCallback, useEffect, useRef, useState } from "react";
import type { UpdateInfo } from "@/hooks/useBlurMachine";

interface UpdateProgress {
  phase: string;
  percent: number;
  message: string;
}

interface Props {
  updateInfo: UpdateInfo;
  onDismiss: () => void;
}

export function UpdatePopup({ updateInfo, onDismiss }: Props) {
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [error, setError] = useState<string | null>(null);
  const timerRef = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    const unlisten = listen("update_progress", (event) => {
      setProgress(event.payload as UpdateProgress);
    });
    return () => { unlisten.then((fn: UnlistenFn) => fn()); };
  }, []);

  // Auto-dismiss after 10s
  useEffect(() => {
    timerRef.current = setTimeout(() => {
      onDismiss();
    }, 10_000);
    return () => {
      if (timerRef.current) clearTimeout(timerRef.current);
    };
  }, [onDismiss]);

  const handleUpdate = useCallback(async () => {
    if (timerRef.current) clearTimeout(timerRef.current);
    try {
      setError(null);
      await invoke("start_update");
    } catch (e) {
      setError(String(e));
      setProgress(null);
    }
  }, []);

  const handleDismiss = useCallback(() => {
    if (timerRef.current) clearTimeout(timerRef.current);
    onDismiss();
  }, [onDismiss]);

  return (
    <div
      className="absolute inset-0 z-50 grid place-items-center"
      style={{
        background: "color-mix(in oklab, black 55%, transparent)",
        backdropFilter: "blur(7px) saturate(120%)",
        animation: "blur-in 300ms ease both",
      }}
      role="dialog"
      aria-modal="true"
      aria-label="Update available"
    >
      <div
        className="cyber-panel relative w-full max-w-[340px] rounded-xl px-5 pb-5 pt-5"
        style={{ animation: "modal-in 300ms cubic-bezier(0.16,1,0.3,1) both" }}
      >
        {/* top hairline */}
        <span
          className="absolute inset-x-6 top-0 h-px"
          style={{ background: "linear-gradient(90deg, transparent, var(--accent-bright), transparent)" }}
        />

        <header className="mb-4 flex items-baseline justify-between">
          <h2 className="font-display text-[13px] font-semibold tracking-[0.3em] text-white/90 text-glow">
            UPDATE AVAILABLE
          </h2>
        </header>

        {!progress ? (
          <>
            <p className="mb-1 font-mono-cyber text-[10.5px] tracking-[0.18em] text-white/45">
              A NEW VERSION IS READY TO INSTALL
            </p>
            <p className="mb-5 font-mono-cyber text-[10px] tracking-[0.14em] text-white/30">
              v{updateInfo.current_version} → v{updateInfo.latest_version}
            </p>

            <div className="flex gap-3">
              <button
                onClick={handleUpdate}
                className="flex-1 rounded-md py-2.5 font-display text-[11px] font-semibold tracking-[0.28em] text-white/90 transition-all hover:brightness-110"
                style={{
                  border: "1px solid color-mix(in oklab, var(--accent) 50%, transparent)",
                  background: "color-mix(in oklab, var(--accent) 18%, transparent)",
                }}
              >
                UPDATE
              </button>
              <button
                onClick={handleDismiss}
                className="flex-1 rounded-md py-2.5 font-display text-[11px] font-semibold tracking-[0.28em] text-white/50 transition-all hover:text-white/70"
                style={{
                  border: "1px solid color-mix(in oklab, white 10%, transparent)",
                  background: "color-mix(in oklab, white 4%, transparent)",
                }}
              >
                DISMISS
              </button>
            </div>

            {error && (
              <p className="mt-3 font-mono-cyber text-[8px] text-red-400/60 truncate" title={error}>
                {error.length > 50 ? error.slice(0, 50) + "..." : error}
              </p>
            )}
          </>
        ) : (
          <>
            <p className="mb-3 font-mono-cyber text-[10.5px] tracking-[0.18em] text-white/45">
              {progress.message}
            </p>

            {progress.phase === "downloading" && (
              <div className="mb-3">
                <div className="mb-2 flex items-center justify-between font-mono-cyber text-[9.5px] tracking-[0.28em] text-white/40">
                  <span>DOWNLOADING</span>
                  <span>{Math.round(progress.percent)}%</span>
                </div>
                <div
                  className="h-1.5 overflow-hidden rounded-full"
                  style={{ background: "color-mix(in oklab, var(--accent) 12%, transparent)" }}
                >
                  <div
                    className="h-full rounded-full"
                    style={{
                      width: `${progress.percent}%`,
                      background: "linear-gradient(90deg, var(--accent-deep), var(--accent-bright))",
                      boxShadow: "0 0 14px var(--accent)",
                      transition: "width 260ms linear",
                    }}
                  />
                </div>
              </div>
            )}

            {progress.phase !== "downloading" && (
              <div className="flex items-center gap-2">
                <span
                  className="h-1.5 w-1.5 rounded-full animate-pulse"
                  style={{ background: "var(--accent-bright)" }}
                />
                <span className="font-mono-cyber text-[9.5px] tracking-[0.22em] text-white/50">
                  {progress.phase.toUpperCase()}...
                </span>
              </div>
            )}
          </>
        )}
      </div>
    </div>
  );
}
