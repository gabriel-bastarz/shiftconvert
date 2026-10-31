import { BrandOrbit } from "../BrandOrbit";
import { StepHeader } from "../StepHeader";

export function WelcomeStep() {
  return (
    <div className="onboarding-step onboarding-step-hero">
      <BrandOrbit />
      <StepHeader
        eyebrow="Bem-vindo ao ShiftConvert"
        title={
          <>
            Converta arquivos.{" "}
            <span className="onboarding-title-accent">
              Sem perder tempo.
            </span>
          </>
        }
        description="Transforme qualquer arquivo em poucos segundos. Sem abrir menus, procurar opções ou interromper seu fluxo de trabalho."
      />
      <div className="onboarding-welcome-hint">
        <span className="onboarding-hint-line" />
        <span>Shift + arraste e pronto.</span>
        <span className="onboarding-hint-line" />
      </div>
    </div>
  );
}
