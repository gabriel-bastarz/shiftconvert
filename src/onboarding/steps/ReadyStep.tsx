import { BrandOrbit } from "../BrandOrbit";
import { StepHeader } from "../StepHeader";

export function ReadyStep() {
  return (
    <div className="onboarding-step onboarding-step-hero">
      <BrandOrbit variant="ready" />
      <StepHeader
        eyebrow="Tudo pronto"
        title={
          <>
            Você está pronto{" "}
            <span className="onboarding-title-accent">para começar.</span>
          </>
        }
        description="Tudo pronto. Seu fluxo de trabalho acaba de ficar mais simples."
      />
      <div className="onboarding-ready-message">
        <span className="onboarding-ready-dot" aria-hidden="true" />
        <span>Bem-vindo ao ShiftConvert. Shift + arraste quando quiser.</span>
      </div>
      <p className="onboarding-ready-note">
        Você pode rever este guia em <strong>Como usar</strong>, no cartão
        inicial.
      </p>
    </div>
  );
}
