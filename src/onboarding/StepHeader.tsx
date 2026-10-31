import type { ReactNode } from "react";

interface StepHeaderProps {
  eyebrow: string;
  title: ReactNode;
  description: ReactNode;
  className?: string;
}

export function StepHeader({
  eyebrow,
  title,
  description,
  className = "",
}: StepHeaderProps) {
  return (
    <header className={`onboarding-copy ${className}`.trim()}>
      <span className="onboarding-eyebrow">{eyebrow}</span>
      <h1 id="onboarding-title" className="onboarding-title">
        {title}
      </h1>
      <p className="onboarding-description">{description}</p>
    </header>
  );
}
