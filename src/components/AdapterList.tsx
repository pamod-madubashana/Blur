import { AdapterRow } from "./AdapterRow";
import type { AdapterProgress } from "@/types/adapter";

export function AdapterList({
  items,
  mode,
}: {
  items: AdapterProgress[];
  mode: "disable" | "enable";
}) {
  return (
    <ul className="flex flex-col gap-1.5">
      {items.map((item, i) => (
        <AdapterRow key={item.adapter.id} item={item} index={i} mode={mode} />
      ))}
    </ul>
  );
}
