import { useMemo, useState } from "react";

import { FormatWheel } from "../components/FormatWheel";
import type { TargetFormat } from "../core/types";
import {
  DEMO_CATEGORIES,
  type DemoCategoryId,
  buildDemoWheelPayload,
} from "./onboarding-demo";
import {
  ArchiveIcon,
  AudioIcon,
  DocumentIcon,
  ImageIcon,
  VideoIcon,
} from "./OnboardingIcons";
import { useElementSize } from "./useElementSize";

function CategoryIcon({ id, size = 15 }: { id: DemoCategoryId; size?: number }) {
  switch (id) {
    case "document":
      return <DocumentIcon size={size} />;
    case "image":
      return <ImageIcon size={size} />;
    case "audio":
      return <AudioIcon size={size} />;
    case "video":
      return <VideoIcon size={size} />;
    case "archive":
      return <ArchiveIcon size={size} />;
  }
}

export function OrbitDemo() {
  const [categoryId, setCategoryId] = useState<DemoCategoryId>("document");
  const [hovered, setHovered] = useState<number | null>(null);
  const [selectedTarget, setSelectedTarget] = useState<TargetFormat | null>(null);
  const [wheelRef, wheelSize] = useElementSize<HTMLDivElement>();

  const category =
    DEMO_CATEGORIES.find((item) => item.id === categoryId) ?? DEMO_CATEGORIES[0];
  const size = Math.min(wheelSize.width, wheelSize.height);
  const payload = useMemo(() => {
    if (size <= 0) return null;
    return buildDemoWheelPayload({
      file: category.file,
      targets: category.targets,
      size,
      mode: "picker",
    });
  }, [category, size]);

  const detail =
    hovered !== null ? category.targets[hovered] ?? null : selectedTarget;

  const selectCategory = (id: DemoCategoryId) => {
    if (id === categoryId) return;
    setCategoryId(id);
    setHovered(null);
    setSelectedTarget(null);
  };

  return (
    <div className="orbit-demo">
      <div className="orbit-wheel-frame" ref={wheelRef}>
        {payload ? (
          <div key={categoryId} className="orbit-wheel-swap">
            <FormatWheel
              payload={payload}
              hovered={hovered}
              onHover={setHovered}
              onSelect={(target) => setSelectedTarget(target)}
            />
          </div>
        ) : null}
      </div>

      <div className="orbit-categories" role="group" aria-label="Famílias de arquivos">
        {DEMO_CATEGORIES.map((item) => (
          <button
            key={item.id}
            type="button"
            className={`orbit-category ${
              item.id === categoryId ? "is-active" : ""
            }`}
            aria-pressed={item.id === categoryId}
            onClick={() => selectCategory(item.id)}
          >
            <span className="orbit-category-icon">
              <CategoryIcon id={item.id} />
            </span>
            <span>{item.label}</span>
          </button>
        ))}
      </div>

      <p className={`orbit-detail ${detail ? "is-active" : ""}`} aria-live="polite">
        {detail ? (
          <>
            <strong>{detail.label}</strong>
            <span>{detail.description}</span>
          </>
        ) : (
          <span>Passe o mouse sobre um formato para ver os detalhes.</span>
        )}
      </p>

      <p className="orbit-category-note">{category.note}</p>
    </div>
  );
}
