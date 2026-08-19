import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";

interface UpdateInfo {
  current_version: string;
  latest_version: string;
  download_url: string;
  digest: string;
  size: number;
}

interface UpdateProgress {
  phase: string;
  percent: number;
  message: string;
}

export function UpdateIndicator() {
  const [updateInfo, setUpdateInfo] = useState<UpdateInfo | null>(null);
  const [progress, setProgress] = useState<UpdateProgress | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    const unlistenAvailable = listen("update_available", (event) => {
      setUpdateInfo(event.payload as UpdateInfo);
    });

    const unlistenProgress = listen("update_progress", (event) => {
      setProgress(event.payload as UpdateProgress);
    });

    return () => {
      unlistenAvailable.then((fn: UnlistenFn) => fn());
      unlistenProgress.then((fn: UnlistenFn) => fn());
    };
  }, []);

  const handleUpdate = async () => {
    try {
      setError(null);
      await invoke("start_update");
    } catch (e) {
      setError(String(e));
      setProgress(null);
    }
  };

  // No update available, no progress — show nothing
  if (!updateInfo && !progress) return null;

  // Download/install in progress
  if (progress) {
    return (
      <div className="absolute bottom-14 left-0 right-0 z-40 flex justify-center px-7">
        <div
          className="cyber-panel w-full max-w-[340px] rounded-lg px-4 py-3"
          style={{ background: "color-mix(in oklab, #0a0a1a 85%, transparent)" }}
        >
          <div className="mb-2 flex items-center justify-between">
            <span className="font-mono-cyber text-[10px] tracking-wider text-white/60">
              {progress.message}
            </span>
            {progress.phase === "downloading" && (
              <span className="font-mono-cyber text-[10px] text-cyan-400/80">
                {Math.round(progress.percent)}%
              </span>
            )}
          </div>
          {progress.phase === "downloading" && (
            <div className="h-1 w-full overflow-hidden rounded-full bg-white/10">
              <div
                className="h-full rounded-full transition-all duration-300"
                style={{
                  width: `${progress.percent}%`,
                  background: "linear-gradient(90deg, #06b6d4, #3b82f6)",
                }}
              />
            </div>
          )}
        </div>
      </div>
    );
  }

  // Update available — show badge
  if (updateInfo) {
    return (
      <div className="absolute top-6 right-14 z-40 flex items-center gap-2">
        <button
          onClick={handleUpdate}
          className="group flex items-center gap-2 rounded-lg border border-cyan-500/20 bg-cyan-500/5 px-3 py-1.5 transition-all hover:border-cyan-500/40 hover:bg-cyan-500/10"
        >
          <span className="inline-block h-1.5 w-1.5 rounded-full bg-cyan-400 animate-pulse" />
          <span className="font-mono-cyber text-[9.5px] tracking-wider text-cyan-400/70 group-hover:text-cyan-400/90">
            Update available
          </span>
        </button>
        {error && (
          <span className="font-mono-cyber text-[8px] text-red-400/60 max-w-[200px] truncate" title={error}>
            {error.length > 40 ? error.slice(0, 40) + "..." : error}
          </span>
        )}
      </div>
    );
  }

  return null;
}
