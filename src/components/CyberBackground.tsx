export function CyberBackground() {
  return (
    <div className="pointer-events-none absolute inset-0 overflow-hidden">
      {/* deep vignette wash */}
      <div
        className="absolute inset-0"
        style={{
          background:
            "radial-gradient(120% 90% at 50% 40%, color-mix(in oklab, var(--accent) 16%, transparent) 0%, transparent 62%), radial-gradient(80% 70% at 50% 120%, color-mix(in oklab, var(--accent-deep) 30%, transparent) 0%, transparent 70%)",
        }}
      />
      {/* technical grid */}
      <div
        className="absolute inset-0 opacity-[0.35]"
        style={{
          backgroundImage:
            "linear-gradient(color-mix(in oklab, var(--accent) 12%, transparent) 1px, transparent 1px), linear-gradient(90deg, color-mix(in oklab, var(--accent) 12%, transparent) 1px, transparent 1px)",
          backgroundSize: "44px 44px",
          maskImage:
            "radial-gradient(circle at 50% 45%, black 8%, transparent 78%)",
        }}
      />
      {/* horizon lines */}
      <div
        className="absolute inset-x-0 bottom-0 h-40 opacity-40"
        style={{
          backgroundImage:
            "repeating-linear-gradient(180deg, color-mix(in oklab, var(--accent) 22%, transparent) 0 1px, transparent 1px 14px)",
          maskImage: "linear-gradient(to top, black, transparent)",
        }}
      />
      {/* floating particles */}
      {PARTICLES.map((p, i) => (
        <span
          key={i}
          className="absolute rounded-full"
          style={{
            left: `${p.x}%`,
            top: `${p.y}%`,
            width: p.s,
            height: p.s,
            background: "var(--accent-bright)",
            boxShadow: "0 0 10px var(--accent)",
            opacity: p.o,
            animation: `drift ${p.d}s ease-in-out ${p.delay}s infinite`,
          }}
        />
      ))}
      {/* corner brackets */}
      {[
        "left-5 top-5 border-l border-t",
        "right-5 top-5 border-r border-t",
        "left-5 bottom-5 border-l border-b",
        "right-5 bottom-5 border-r border-b",
      ].map((c) => (
        <div
          key={c}
          className={`absolute h-5 w-5 ${c}`}
          style={{ borderColor: "color-mix(in oklab, var(--accent) 45%, transparent)" }}
        />
      ))}
    </div>
  );
}

const PARTICLES = [
  { x: 12, y: 24, s: 3, o: 0.5, d: 7, delay: 0 },
  { x: 22, y: 70, s: 2, o: 0.35, d: 9, delay: 1.2 },
  { x: 34, y: 15, s: 2, o: 0.4, d: 8, delay: 0.6 },
  { x: 68, y: 30, s: 3, o: 0.45, d: 10, delay: 2 },
  { x: 82, y: 62, s: 2, o: 0.4, d: 7.5, delay: 0.9 },
  { x: 90, y: 20, s: 2, o: 0.3, d: 11, delay: 1.7 },
  { x: 56, y: 84, s: 3, o: 0.35, d: 8.5, delay: 2.4 },
  { x: 44, y: 52, s: 2, o: 0.25, d: 12, delay: 0.3 },
];
