import { useMemo } from "react";

import type { TargetFormat, WheelLayout, WheelPayload } from "../core/types";

const TAU = Math.PI * 2;

type Point = [number, number];

interface FormatWheelProps {
  payload: WheelPayload;
  hovered: number | null;
  onHover: (index: number | null) => void;
  onSelect: (target: TargetFormat) => void;
}

function pointFrom(
  layout: WheelLayout,
  angle: number,
  radius: number
): Point {
  return [
    layout.centerX + Math.cos(angle) * radius,
    layout.centerY + Math.sin(angle) * radius,
  ];
}

function seamAngle(layout: WheelLayout, index: number): number {
  return layout.rotation + (index * TAU) / Math.max(1, layout.seamBends.length);
}

function pathFromPoints(points: Point[]): string {
  if (points.length === 0) return "";
  const round = (value: number) => Math.round(value * 100) / 100;
  const [first, ...rest] = points;
  let path = `M ${round(first[0])} ${round(first[1])}`;
  for (const point of rest) {
    path += ` L ${round(point[0])} ${round(point[1])}`;
  }
  return `${path} Z`;
}

function circlePath(cx: number, cy: number, radius: number): string {
  const r = Math.round(radius * 100) / 100;
  const x = Math.round(cx * 100) / 100;
  const y = Math.round(cy * 100) / 100;
  return `M ${x + r} ${y} A ${r} ${r} 0 1 0 ${x - r} ${y} A ${r} ${r} 0 1 0 ${x + r} ${y} Z`;
}

function donutPath(
  cx: number,
  cy: number,
  outer: number,
  inner: number
): string {
  return `${circlePath(cx, cy, outer)} ${circlePath(cx, cy, inner)}`;
}

/**
 * Pétala radial da Orbit Minimal. Cada pétala é um setor circular: arco
 * externo, duas laterais rectas e radiais, e arco interno. O núcleo
 * permanece um círculo perfeito e as divisórias entre pétalas são linhas
 * retas do centro até o aro externo, sem curvatura ou ondulação.
 */
function petalPath(
  layout: WheelLayout,
  index: number,
  count: number,
  regionInner: number,
  regionOuter: number,
  gapAngleHalf: number
): string {
  const points: Point[] = [];

  const startSeam = index;
  const endSeam = (index + 1) % count;

  const span = TAU / count;
  const effectiveGap = Math.min(gapAngleHalf, span * 0.49);
  const startAngle = seamAngle(layout, startSeam) + effectiveGap;
  const endAngle = seamAngle(layout, endSeam) - effectiveGap;

  // 1. Arco externo circular: startAngle -> endAngle (sentido anti-horário)
  const outerSteps = 24;
  for (let step = 0; step <= outerSteps; step += 1) {
    const angle = startAngle + ((endAngle - startAngle) * step) / outerSteps;
    points.push(pointFrom(layout, angle, regionOuter));
  }

  // 2. Lateral direita reta (radial): endAngle, de regionOuter até regionInner
  const sideSteps = 8;
  for (let step = 1; step <= sideSteps; step += 1) {
    const radius =
      regionOuter - ((regionOuter - regionInner) * step) / sideSteps;
    points.push(pointFrom(layout, endAngle, radius));
  }

  // 3. Arco interno circular: endAngle -> startAngle (sentido horário)
  const innerSteps = 20;
  for (let step = 0; step <= innerSteps; step += 1) {
    const angle =
      endAngle - ((endAngle - startAngle) * step) / innerSteps;
    points.push(pointFrom(layout, angle, regionInner));
  }

  // 4. Lateral esquerda reta (radial): startAngle, de regionInner até regionOuter
  for (let step = 1; step < sideSteps; step += 1) {
    const radius =
      regionInner + ((regionOuter - regionInner) * step) / sideSteps;
    points.push(pointFrom(layout, startAngle, radius));
  }

  return pathFromPoints(points);
}

function formatFileSize(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return "";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let value = bytes;
  let unit = 0;
  while (value >= 1024 && unit < units.length - 1) {
    value /= 1024;
    unit += 1;
  }
  const digits = unit === 0 ? 0 : value >= 10 ? 1 : 1;
  const formatted = value.toFixed(digits).replace(".", ",");
  return `${formatted} ${units[unit]}`;
}

export function FormatWheel({
  payload,
  hovered,
  onHover,
  onSelect,
}: FormatWheelProps) {
  const { layout, targets, file, overlay } = payload;
  const width = overlay.windowWidth / overlay.scale;
  const height = overlay.windowHeight / overlay.scale;
  const cx = layout.centerX;
  const cy = layout.centerY;
  const count = Math.max(1, targets.length);
  const interactive = payload.mode === "picker";

  const geometry = useMemo(() => {
    const outerRadius = layout.outerRadius;
    const hubRadius = layout.innerRadius;
    const rimGap = Math.max(3.8, outerRadius * 0.058);
    const hubGap = Math.max(4.9, outerRadius * 0.07);
    const panelGap = Math.max(4.2, outerRadius * 0.054);
    const regionOuter = outerRadius - rimGap;
    const regionInner = Math.min(
      regionOuter - 8,
      hubRadius + hubGap
    );
    const gapHalf = panelGap / 2;
    const gapAngleHalf = gapHalf / Math.max(regionOuter, 1);

    const outerPath = circlePath(cx, cy, outerRadius);
    const hubPath = circlePath(cx, cy, hubRadius);
    const regions =
      count === 1
        ? [donutPath(cx, cy, regionOuter, regionInner)]
        : targets.map((_, index) =>
            petalPath(
              layout,
              index,
              count,
              regionInner,
              regionOuter,
              gapAngleHalf
            )
          );

    const labels = targets.map((target, index) => {
      let angle = -Math.PI / 2;
      if (count > 1) {
        let start = seamAngle(layout, index);
        let end = seamAngle(layout, (index + 1) % count);
        while (end <= start) end += TAU;
        angle = (start + end) / 2;
      }
      const radius = (regionInner + regionOuter) / 2;
      const [x, y] = pointFrom(layout, angle, radius);
      return { x, y, target };
    });

    return {
      outerPath,
      hubPath,
      regions,
      labels,
      regionInner,
      regionOuter,
      rimGap,
      hubGap,
      hubRadius,
    };
  }, [count, cx, cy, layout, targets]);

  const fontSize = Math.max(
    6.9,
    Math.min(11.6, layout.outerRadius * (count > 6 ? 0.095 : 0.11))
  );
  const centerFontSize = Math.max(
    7.2,
    Math.min(12, geometry.hubRadius * 0.28)
  );
  const sizeFontSize = Math.max(
    6,
    Math.min(9, geometry.hubRadius * 0.21)
  );

  const maxNameChars = Math.max(
    7,
    Math.floor(
      (geometry.hubRadius * 1.9) / (centerFontSize * 0.55)
    )
  );
  const fileName =
    file.fileName.length > maxNameChars
      ? `${file.fileName.slice(0, maxNameChars - 1)}…`
      : file.fileName;
  const sizeText = formatFileSize(file.sizeBytes);

  return (
    <svg
      className="wheel-svg orbit-wheel"
      viewBox={`0 0 ${width} ${height}`}
      width="100%"
      height="100%"
      role="group"
      aria-label={`Formatos de saída para ${file.fileName}`}
      onPointerLeave={() => onHover(null)}
    >
      <defs>
        <radialGradient
          id="orbit-disc"
          cx={cx}
          cy={cy - layout.outerRadius * 0.28}
          r={layout.outerRadius * 1.22}
          gradientUnits="userSpaceOnUse"
        >
          <stop offset="0" stopColor="#1b2430" />
          <stop offset="0.56" stopColor="#111821" />
          <stop offset="1" stopColor="#080c11" />
        </radialGradient>

        <linearGradient
          id="orbit-panel"
          x1={cx - layout.outerRadius}
          y1={cy - layout.outerRadius}
          x2={cx + layout.outerRadius}
          y2={cy + layout.outerRadius}
          gradientUnits="userSpaceOnUse"
        >
          <stop offset="0" stopColor="#222d39" />
          <stop offset="0.55" stopColor="#161f29" />
          <stop offset="1" stopColor="#111820" />
        </linearGradient>

        <linearGradient
          id="orbit-hover"
          x1={cx}
          y1={cy - layout.outerRadius}
          x2={cx}
          y2={cy + layout.outerRadius}
          gradientUnits="userSpaceOnUse"
        >
          <stop offset="0" stopColor="#1d619c" />
          <stop offset="0.45" stopColor="#143f68" />
          <stop offset="1" stopColor="#0b2a45" />
        </linearGradient>

        <linearGradient
          id="orbit-hub"
          x1={cx - geometry.hubRadius}
          y1={cy - geometry.hubRadius}
          x2={cx + geometry.hubRadius}
          y2={cy + geometry.hubRadius}
          gradientUnits="userSpaceOnUse"
        >
          <stop offset="0" stopColor="#1a2430" />
          <stop offset="0.6" stopColor="#111922" />
          <stop offset="1" stopColor="#0c1218" />
        </linearGradient>

        <filter
          id="orbit-hub-shadow"
          x="-40%"
          y="-40%"
          width="180%"
          height="180%"
        >
          <feGaussianBlur stdDeviation="2.6" />
        </filter>
      </defs>

      <g
        className="wheel-group"
        style={{
          transformOrigin: `${cx}px ${cy}px`,
          filter: "drop-shadow(0 12px 28px rgba(0, 0, 0, 0.58))",
        }}
      >
        <path d={geometry.outerPath} fill="url(#orbit-disc)" />

        <g>
          {targets.map((target, index) => (
            <path
              key={`orbit-panel-${target.id}`}
              d={geometry.regions[index]}
              fill="url(#orbit-panel)"
              fillRule="evenodd"
              stroke="#303c4b"
              strokeWidth={1.05}
              strokeLinejoin="round"
              pointerEvents="none"
            />
          ))}
        </g>

        {hovered !== null && hovered >= 0 && hovered < targets.length ? (
          <path
            className="orbit-hover-path"
            d={geometry.regions[hovered]}
            fill="url(#orbit-hover)"
            fillRule="evenodd"
            stroke="#4aa3ff"
            strokeWidth={1.6}
            strokeLinejoin="round"
            pointerEvents="none"
            style={{
              filter:
                "drop-shadow(0 0 7px rgba(58, 156, 255, 0.55))",
              transformOrigin: `${cx}px ${cy}px`,
            }}
          />
        ) : null}

        <path
          d={geometry.outerPath}
          fill="none"
          stroke="#455261"
          strokeWidth={1.3}
          pointerEvents="none"
        />
        <circle
          cx={cx}
          cy={cy}
          r={Math.max(1, layout.outerRadius - 3.1)}
          fill="none"
          stroke="#1a222b"
          strokeWidth={0.9}
          pointerEvents="none"
        />
        <circle
          cx={cx}
          cy={cy}
          r={Math.max(1, layout.outerRadius - 1.1)}
          fill="none"
          stroke="rgba(255,255,255,0.045)"
          strokeWidth={0.7}
          pointerEvents="none"
        />

        <circle
          cx={cx}
          cy={cy + geometry.hubRadius * 0.05}
          r={geometry.hubRadius + 3.4}
          fill="rgba(0, 0, 0, 0.5)"
          filter="url(#orbit-hub-shadow)"
          pointerEvents="none"
        />
        <path
          d={geometry.hubPath}
          fill="url(#orbit-hub)"
          stroke="#354151"
          strokeWidth={1.1}
          pointerEvents="none"
        />
        <circle
          cx={cx}
          cy={cy}
          r={Math.max(1, geometry.hubRadius - 2.2)}
          fill="none"
          stroke="rgba(255,255,255,0.045)"
          strokeWidth={0.8}
          pointerEvents="none"
        />

        {geometry.labels.map((item, index) => {
          const isHovered = hovered === index;
          return (
            <text
              key={`orbit-label-${item.target.id}`}
              x={item.x}
              y={item.y}
              textAnchor="middle"
              dominantBaseline="middle"
              fill={isHovered ? "#ffffff" : "#d9e3ee"}
              fontSize={isHovered ? fontSize * 1.035 : fontSize}
              fontWeight={700}
              letterSpacing="0.055em"
              pointerEvents="none"
              style={{ transition: "fill 140ms ease" }}
            >
              {item.target.label}
            </text>
          );
        })}

        <text
          x={cx}
          y={cy - geometry.hubRadius * (sizeText ? 0.13 : 0)}
          textAnchor="middle"
          dominantBaseline="middle"
          fill="#f0f5fb"
          fontSize={centerFontSize}
          fontWeight={600}
          letterSpacing="-0.01em"
          pointerEvents="none"
        >
          {fileName}
        </text>
        {sizeText ? (
          <text
            x={cx}
            y={cy + geometry.hubRadius * 0.27}
            textAnchor="middle"
            dominantBaseline="middle"
            fill="#8492a3"
            fontSize={sizeFontSize}
            fontWeight={500}
            pointerEvents="none"
          >
            {sizeText}
          </text>
        ) : null}

        {interactive
          ? targets.map((target, index) => (
              <path
                key={`orbit-hit-${target.id}`}
                d={geometry.regions[index]}
                fill="rgba(255,255,255,0.001)"
                fillRule="evenodd"
                stroke="none"
                role="button"
                tabIndex={0}
                aria-label={`Converter para ${target.label}: ${target.description}`}
                style={{ cursor: "pointer" }}
                onPointerEnter={() => onHover(index)}
                onPointerLeave={() => onHover(null)}
                onFocus={() => onHover(index)}
                onBlur={() => onHover(null)}
                onClick={() => onSelect(target)}
                onKeyDown={(event) => {
                  if (event.key === "Enter" || event.key === " ") {
                    event.preventDefault();
                    onSelect(target);
                  }
                }}
              />
            ))
          : null}
      </g>
    </svg>
  );
}
