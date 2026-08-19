import { useState } from "react";
import { MorphLabel } from "./MorphLabel";
import type { AppState } from "@/types/adapter";

interface Props {
  state: AppState;
  onActivate: () => void;
  disabled?: boolean;
}

export function BlurControl({ state, onActivate, disabled }: Props) {
  const [hover, setHover] = useState(false);
  const interactive = (state === "ready" || state === "running") && !disabled;
  const running = state === "running" || state === "enabling";

  const base = running ? "RUNNING" : "BLUR";
  const hovered = running ? "STOP" : "START";
  const label = interactive && hover ? hovered : base;

  const sub =
    state === "disabling"
      ? "WORKING"
      : state === "enabling"
        ? "WORKING"
        : interactive && hover
          ? running
            ? "RESTORE ADAPTERS"
            : "ISOLATE ADAPTERS"
          : running
            ? "SYSTEM ACTIVE"
            : "SYSTEM IDLE";

  return (
    <button
      type="button"
      disabled={!interactive}
      onMouseEnter={() => setHover(true)}
      onMouseLeave={() => setHover(false)}
      onClick={() => interactive && onActivate()}
      aria-label={label}
      className="group relative grid h-[clamp(232px,38vh,286px)] w-[clamp(232px,38vh,286px)] place-items-center rounded-full outline-none transition-transform duration-300 ease-out disabled:cursor-default"
      style={{
        transform: interactive && hover ? "scale(1.025)" : "scale(1)",
        opacity: interactive ? 1 : 0.72,
      }}
    >
      {/* outer glow */}
      <span
        className="pointer-events-none absolute inset-[-22%] rounded-full"
        style={{
          background:
            "radial-gradient(circle, color-mix(in oklab, var(--accent) 40%, transparent) 0%, transparent 66%)",
          animation: "blur-pulse 4.5s ease-in-out infinite",
        }}
      />
      {/* radial ripple */}
      <span
        className="pointer-events-none absolute inset-0 rounded-full"
        style={{
          border: "1px solid color-mix(in oklab, var(--accent) 45%, transparent)",
          animation: "blur-ripple 3.6s cubic-bezier(0.2,0.7,0.3,1) infinite",
        }}
      />
      {/* outer rotating dashed ring */}
      <span
        className="pointer-events-none absolute inset-[-8%] rounded-full"
        style={{
          border: "1px dashed color-mix(in oklab, var(--accent) 38%, transparent)",
          animation: "blur-spin 26s linear infinite",
        }}
      />
      {/* segmented ring */}
      <span
        className="pointer-events-none absolute inset-[-2%] rounded-full"
        style={{
          background: `conic-gradient(from 0deg, transparent 0deg 28deg, color-mix(in oklab, var(--accent-bright) 75%, transparent) 34deg 52deg, transparent 58deg 148deg, color-mix(in oklab, var(--accent) 65%, transparent) 154deg 186deg, transparent 192deg 300deg, color-mix(in oklab, var(--accent-bright) 55%, transparent) 306deg 322deg, transparent 328deg 360deg)`,
          WebkitMask: "radial-gradient(farthest-side, transparent calc(100% - 2px), black calc(100% - 2px))",
          mask: "radial-gradient(farthest-side, transparent calc(100% - 2px), black calc(100% - 2px))",
          animation: `blur-spin ${hover && interactive ? "5s" : "12s"} linear infinite`,
        }}
      />
      {/* tick ring */}
      <span
        className="pointer-events-none absolute inset-[6%] rounded-full opacity-70"
        style={{
          background:
            "repeating-conic-gradient(from 0deg, color-mix(in oklab, var(--accent) 55%, transparent) 0deg 0.6deg, transparent 0.6deg 6deg)",
          WebkitMask: "radial-gradient(farthest-side, transparent calc(100% - 7px), black calc(100% - 7px))",
          mask: "radial-gradient(farthest-side, transparent calc(100% - 7px), black calc(100% - 7px))",
          animation: "blur-spin-rev 60s linear infinite",
        }}
      />
      {/* main disc */}
      <span
        className="absolute inset-[12%] overflow-hidden rounded-full"
        style={{
          background:
            "radial-gradient(circle at 50% 30%, color-mix(in oklab, var(--accent) 26%, transparent), color-mix(in oklab, var(--void) 92%, black) 72%)",
          border: "1px solid color-mix(in oklab, var(--accent) 55%, transparent)",
          boxShadow:
            "inset 0 0 60px color-mix(in oklab, var(--accent) 28%, transparent), 0 0 60px -10px color-mix(in oklab, var(--accent) 60%, transparent)",
        }}
      >
        {/* scanline */}
        <span
          className="absolute inset-x-0 h-10"
          style={{
            background:
              "linear-gradient(180deg, transparent, color-mix(in oklab, var(--accent-bright) 22%, transparent), transparent)",
            animation: "blur-scan 5s ease-in-out infinite",
          }}
        />
        {/* inner hairline ring */}
        <span
          className="absolute inset-[14%] rounded-full"
          style={{ border: "1px solid color-mix(in oklab, var(--accent) 26%, transparent)" }}
        />
        <span
          className="absolute inset-[24%] rounded-full"
          style={{
            border: "1px solid color-mix(in oklab, var(--accent) 16%, transparent)",
            animation: "blur-breathe 3.2s ease-in-out infinite",
          }}
        />
      </span>

      {/* orbiting particles */}
      {[0, 1, 2].map((i) => (
        <span
          key={i}
          className="pointer-events-none absolute inset-0"
          style={{ animation: `blur-spin ${9 + i * 5}s linear infinite`, transform: `rotate(${i * 120}deg)` }}
        >
          <span
            className="absolute left-1/2 top-0 h-1.5 w-1.5 -translate-x-1/2 rounded-full"
            style={{ background: "var(--accent-bright)", boxShadow: "0 0 12px var(--accent-bright)" }}
          />
        </span>
      ))}

      {/* label */}
      <span className="relative z-10 flex flex-col items-center gap-2">
        <MorphLabel
          text={label}
          className="font-display text-[34px] font-semibold leading-none tracking-[0.24em] text-white text-glow"
          key="main"
        />
        <span
          className="font-mono-cyber text-[9.5px] tracking-[0.34em] transition-opacity duration-300"
          style={{ color: "color-mix(in oklab, var(--accent-bright) 70%, white)", opacity: 0.65 }}
        >
          {sub}
        </span>
      </span>
    </button>
  );
}
