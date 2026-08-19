export interface NetworkAdapter {
  id: string;
  name: string;
  type: string;
}

export type AdapterPhase = "pending" | "processing" | "done" | "failed";

export interface AdapterProgress {
  adapter: NetworkAdapter;
  phase: AdapterPhase;
}

export interface AdapterService {
  listAdapters(): Promise<NetworkAdapter[]>;
  disableAdapter(id: string): Promise<void>;
  enableAdapter(id: string): Promise<void>;
}

export type AppState = "ready" | "checking" | "disabling" | "launching" | "running" | "enabling";
