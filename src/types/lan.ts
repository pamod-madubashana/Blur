export interface LanPeer {
  address: string;
  port: number;
  name: string;
  strings: string[];
  packets: number;
  last_seen: number;
}

export interface LanSnapshot {
  listening: boolean;
  error: string | null;
  interface: string;
  peers: LanPeer[];
}
