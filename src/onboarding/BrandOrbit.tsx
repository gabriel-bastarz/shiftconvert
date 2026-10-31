import { CheckIcon } from "./OnboardingIcons";

interface BrandOrbitProps {
  variant?: "welcome" | "ready";
}

export function BrandOrbit({ variant = "welcome" }: BrandOrbitProps) {
  return (
    <div className={`brand-orbit brand-orbit-${variant}`} aria-hidden="true">
      <span className="brand-orbit-halo" />
      <span className="brand-orbit-ring brand-orbit-ring-a" />
      <span className="brand-orbit-ring brand-orbit-ring-b" />
      <span className="brand-orbit-satellite brand-orbit-satellite-a" />
      <span className="brand-orbit-satellite brand-orbit-satellite-b" />
      <span className="brand-orbit-logo-frame">
        <img src="/favicon.png" alt="" className="brand-orbit-logo" />
      </span>
      {variant === "ready" ? (
        <span className="brand-orbit-check">
          <CheckIcon size={18} />
        </span>
      ) : null}
    </div>
  );
}
