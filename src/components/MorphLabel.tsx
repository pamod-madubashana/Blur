import { useEffect, useRef, useState } from "react";

export function MorphLabel({ text, className }: { text: string; className?: string }) {
  const [current, setCurrent] = useState(text);
  const [previous, setPrevious] = useState<string | null>(null);
  const timer = useRef<ReturnType<typeof setTimeout> | null>(null);

  useEffect(() => {
    if (text === current) return;
    setPrevious(current);
    setCurrent(text);
    if (timer.current) clearTimeout(timer.current);
    timer.current = setTimeout(() => setPrevious(null), 320);
    return () => {
      if (timer.current) clearTimeout(timer.current);
    };
  }, [text, current]);

  return (
    <span className={`relative grid place-items-center ${className ?? ""}`}>
      <span className="invisible col-start-1 row-start-1">{longest(current, previous)}</span>
      {previous !== null && (
        <span
          key={`out-${previous}`}
          className="col-start-1 row-start-1 whitespace-nowrap"
          style={{ animation: "label-out 280ms cubic-bezier(0.4,0,1,1) forwards" }}
        >
          {previous}
        </span>
      )}
      <span
        key={`in-${current}`}
        className="col-start-1 row-start-1 whitespace-nowrap"
        style={{ animation: "label-in 320ms cubic-bezier(0.16,1,0.3,1) both" }}
      >
        {current}
      </span>
    </span>
  );
}

function longest(a: string, b: string | null) {
  if (!b) return a;
  return a.length >= b.length ? a : b;
}
