import { invoke } from "@tauri-apps/api/core";
import type { AdapterService, NetworkAdapter } from "@/types/adapter";

async function withTimeout<T>(promise: Promise<T>, ms: number): Promise<T> {
  return Promise.race([
    promise,
    new Promise<T>((_, reject) => setTimeout(() => reject(new Error("timeout")), ms)),
  ]);
}

export const adapterService: AdapterService = {
  async listAdapters(): Promise<NetworkAdapter[]> {
    console.log("[adapterService] listAdapters - fetching...");
    const adapters = await withTimeout(
      invoke<Array<{ name: string; status: string; adapter_type: string }>>("list_adapters"),
      10000,
    );
    console.log("[adapterService] listAdapters - got", adapters.length, "adapters:", adapters);
    return adapters.map((a) => ({
      id: a.name,
      name: a.name,
      type: a.adapter_type,
    }));
  },

  async disableAdapter(id: string): Promise<void> {
    console.log("[adapterService] disableAdapter -", id);
    try {
      await withTimeout(invoke("disable_adapter", { name: id }), 8000);
      console.log("[adapterService] disableAdapter -", id, "OK");
    } catch (e) {
      console.error("[adapterService] disableAdapter -", id, "FAILED:", e);
      throw e;
    }
  },

  async enableAdapter(id: string): Promise<void> {
    console.log("[adapterService] enableAdapter -", id);
    try {
      await withTimeout(invoke("enable_adapter", { name: id }), 8000);
      console.log("[adapterService] enableAdapter -", id, "OK");
    } catch (e) {
      console.error("[adapterService] enableAdapter -", id, "FAILED:", e);
      throw e;
    }
  },
};
