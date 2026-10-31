import { GestureDemo } from "../GestureDemo";
import { StepHeader } from "../StepHeader";

export function GestureStep() {
  return (
    <div className="onboarding-step">
      <StepHeader
        eyebrow="O gesto"
        title={
          <>
            Mantenha pressionado.{" "}
            <span className="onboarding-title-accent">Arraste. Converta.</span>
          </>
        }
        description="Segure Shift enquanto arrasta qualquer arquivo para abrir o menu de conversão."
      />
      <GestureDemo />
      <p className="onboarding-footnote">
        O menu aparece exatamente onde o mouse está. Basta soltar sobre o formato desejado.
      </p>
    </div>
  );
}
