import { OrbitDemo } from "../OrbitDemo";
import { StepHeader } from "../StepHeader";

export function OrbitStep() {
  return (
    <div className="onboarding-step">
      <StepHeader
        eyebrow="Seleção inteligente"
        title={
          <>
            Diversos formatos.{" "}
            <span className="onboarding-title-accent">Em um só lugar.</span>
          </>
        }
        description="Escolha entre diferentes opções de conversão em um menu rápido e intuitivo."
      />
      <OrbitDemo />
      <p className="onboarding-footnote">
        No uso real, o menu exibe automaticamente apenas os formatos compatíveis com o arquivo arrastado.
      </p>
    </div>
  );
}
