import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { adapterService } from "@/services/adapterService";
import type { AdapterPhase, AdapterProgress, AppState, NetworkAdapter } from "@/types/adapter";

export function useBlurMachine() {
  const [state, setState] = useState<AppState>("ready");
  const [adapters, setAdapters] = useState<NetworkAdapter[]>([]);
  const [allItems, setAllItems] = useState<AdapterProgress[]>([]);
  const [completed, setCompleted] = useState(false);
  const [closing, setClosing] = useState(false);
  const busy = useRef(false);
  const gamePath = useRef<string | null>(null);
  const adaptersRef = useRef<NetworkAdapter[]>([]);

  useEffect(() => {
    let alive = true;

    async function init() {
      try {
        const savedPath = await invoke<string | null>("get_saved_path");
        if (savedPath && alive) gamePath.current = savedPath;

        const list = await adapterService.listAdapters();
        if (alive) {
          setAdapters(list);
          adaptersRef.current = list;
        }
      } catch {
        // ignore
      }
    }

    init();
    return () => { alive = false; };
  }, []);

  useEffect(() => {
    const unlistenStatus = listen<string>("status", (event) => {
      const s = event.payload;
      console.log("[blurMachine] status:", s);

      if (s === "disabling") {
        setState("disabling");
        setCompleted(false);
        setClosing(false);
      } else if (s === "waiting" || s === "launching" || s === "racing") {
        setState("running");
        setCompleted(true);
        setTimeout(() => {
          setClosing(true);
          setTimeout(() => {
            setClosing(false);
            setCompleted(false);
          }, 280);
        }, 850);
      } else if (s === "restoring") {
        setState("enabling");
        setCompleted(false);
        setClosing(false);
      } else if (s === "idle") {
        setCompleted(true);
        setClosing(true);
        setTimeout(() => {
          setState("ready");
          setCompleted(false);
          setClosing(false);
          setAllItems([]);
          busy.current = false;
        }, 300);
      }
    });

    const unlistenAdapters = listen<string[]>("adapters", (event) => {
      const names = event.payload;
      console.log("[blurMachine] adapters:", names, "current list:", adaptersRef.current.length);
      const matched = adaptersRef.current.filter((a) => names.includes(a.name));
      console.log("[blurMachine] matched:", matched.length);
      setAllItems(matched.map((adapter) => ({
        adapter,
        phase: "pending" as const,
      })));
    });

    const unlistenProgress = listen<{ name: string; phase: string }>("adapter_progress", (event) => {
      const { name, phase } = event.payload;
      setAllItems((prev) =>
        prev.map((item) =>
          item.adapter.name === name
            ? { ...item, phase: phase as AdapterPhase }
            : item,
        ),
      );
    });

    const unlistenFinished = listen("finished", () => {
      console.log("[blurMachine] finished");
    });

    const unlistenLog = listen<string>("log", (event) => {
      console.log("[backend]", event.payload);
    });

    return () => {
      unlistenStatus.then((fn) => fn());
      unlistenAdapters.then((fn) => fn());
      unlistenProgress.then((fn) => fn());
      unlistenFinished.then((fn) => fn());
      unlistenLog.then((fn) => fn());
    };
  }, []);

  const mode = state === "enabling" ? "enable" : "disable";
  const modalOpen = state === "disabling" || state === "enabling";
  const items = allItems.filter((i) => i.adapter.type === "virtual");
  const totalVirtual = items.length;
  const doneVirtual = items.filter((i) => i.phase === "done" || i.phase === "failed").length;
  const overall = totalVirtual > 0 ? (doneVirtual / totalVirtual) * 100 : 0;

  const activate = useCallback(async () => {
    if (busy.current) return;
    if (state !== "ready") return;

    console.log("[blurMachine] activate");
    if (!gamePath.current) {
      try {
        const picked = await invoke<string | null>("pick_game_path");
        if (!picked) return;
        gamePath.current = picked;
      } catch {
        return;
      }
    }

    busy.current = true;
    invoke("start_lan_mode", { gamePath: gamePath.current }).catch((e) => {
      console.error("[blurMachine] start_lan_mode failed:", e);
      busy.current = false;
      setState("ready");
    });
  }, [state]);

  return { state, items, mode, modalOpen, overall, completed, closing, activate } as const;
}
