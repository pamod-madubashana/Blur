import { useCallback, useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { adapterService } from "@/services/adapterService";
import type { AdapterPhase, AdapterProgress, AppState, NetworkAdapter } from "@/types/adapter";

export interface FileCheckItem {
  file: string;
  status: "ok" | "copying" | "copied";
}

export interface FirewallCheckItem {
  rule: string;
  status: "ok" | "creating" | "created" | "checking";
}

export function useBlurMachine() {
  const [state, setState] = useState<AppState>("ready");
  const [adapters, setAdapters] = useState<NetworkAdapter[]>([]);
  const [allItems, setAllItems] = useState<AdapterProgress[]>([]);
  const [completed, setCompleted] = useState(false);
  const [closing, setClosing] = useState(false);
  const [fileItems, setFileItems] = useState<FileCheckItem[]>([]);
  const [fileCheckDone, setFileCheckDone] = useState(false);
  const [firewallItems, setFirewallItems] = useState<FirewallCheckItem[]>([]);
  const [firewallCheckDone, setFirewallCheckDone] = useState(false);
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

      if (s === "checking") {
        setState("checking");
        setCompleted(false);
        setClosing(false);
        setFileItems([]);
        setFileCheckDone(false);
        setFirewallItems([]);
        setFirewallCheckDone(false);
      } else if (s === "firewall") {
        setState("checking");
        setFileCheckDone(true);
      } else if (s === "disabling") {
        setState("disabling");
        setCompleted(false);
        setClosing(false);
      } else if (s === "waiting" || s === "launching") {
        setState("launching");
        setClosing(false);
      } else if (s === "racing") {
        setState("running");
        setClosing(true);
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
          setFileItems([]);
          setFileCheckDone(false);
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

    const unlistenFileCheck = listen<{ file: string; status: string }>("file_check", (event) => {
      const { file, status } = event.payload;
      setFileItems((prev) => {
        const existing = prev.find((i) => i.file === file);
        if (existing) {
          return prev.map((i) => i.file === file ? { ...i, status: status as FileCheckItem["status"] } : i);
        }
        return [...prev, { file, status: status as FileCheckItem["status"] }];
      });
    });

    const unlistenFileCheckDone = listen<boolean>("file_check_done", () => {
      setFileCheckDone(true);
    });

    const unlistenFirewallCheck = listen<{ rule: string; status: string }>("firewall_check", (event) => {
      const { rule, status } = event.payload;
      setFirewallItems((prev) => {
        const existing = prev.find((i) => i.rule === rule);
        if (existing) {
          return prev.map((i) => i.rule === rule ? { ...i, status: status as FirewallCheckItem["status"] } : i);
        }
        return [...prev, { rule, status: status as FirewallCheckItem["status"] }];
      });
    });

    const unlistenFirewallCheckDone = listen("firewall_check_done", () => {
      setFirewallCheckDone(true);
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
      unlistenFileCheck.then((fn) => fn());
      unlistenFileCheckDone.then((fn) => fn());
      unlistenFirewallCheck.then((fn) => fn());
      unlistenFirewallCheckDone.then((fn) => fn());
      unlistenFinished.then((fn) => fn());
      unlistenLog.then((fn) => fn());
    };
  }, []);

  const mode = state === "enabling" ? "enable" : "disable";
  const adapterModalOpen = state === "disabling" || state === "enabling";
  const launchModalOpen = state === "launching";
  const fileCheckModalOpen = state === "checking";
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

  return { state, items, mode, adapterModalOpen, launchModalOpen, fileCheckModalOpen, fileItems, fileCheckDone, firewallItems, firewallCheckDone, overall, completed, closing, activate } as const;
}
