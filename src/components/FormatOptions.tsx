/**
 * FormatOptions.tsx
 * Mantido como componente separado para compatibilidade com a estrutura
 * proposta. Atualmente, as opções de formato são renderizadas dentro de
 * RadialPetal (submenu). Este componente reexporta a lógica de formatação
 * para uso futuro ou consumo direto.
 */

import { FormatOption } from "@/core/format-registry";

export interface FormatOptionsProps {
  formats: FormatOption[];
  categoryName: string;
  onSelect: (format: string) => void;
}

export const FormatOptions: React.FC<FormatOptionsProps> = ({
  formats,
  categoryName,
  onSelect,
}) => {
  return (
    <div className="format-submenu" data-open="true">
      <div className="format-title">{categoryName}</div>
      <div className="format-hint">Selecione o formato de destino</div>
      <div className="format-options">
        {formats.map((fmt) => (
          <button
            key={fmt.value}
            type="button"
            className="format-option"
            data-implemented={fmt.implemented ? "true" : "false"}
            title={
              fmt.implemented
                ? `Converter para ${fmt.label}`
                : `${fmt.label} (simulação — ainda não implementado)`
            }
            onClick={() => onSelect(fmt.value)}
          >
            {fmt.value.toUpperCase()}
          </button>
        ))}
      </div>
    </div>
  );
};
