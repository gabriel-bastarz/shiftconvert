/**
 * RadialPetal.tsx
 * Componente de uma única pétala do menu radial.
 * Controla estados: hover, ativo, aberto (submenu visível).
 * Renderiza ícone, label e, quando ativo, o submenu de formatos.
 */

import React from "react";
import { FormatOption } from "@/core/format-registry";

export interface RadialPetalProps {
  /** Posição angular da pétala em graus. */
  angle: number;
  /** Índice da pétala (0..4). */
  index: number;
  /** Nome da categoria (ex: "Imagens"). */
  categoryName: string;
  /** Categoria técnica (ex: "image"). */
  category: string;
  /** Emoji/ícone da pétala. */
  icon: string;
  /** Se a pétala está ativa (selecionada). */
  isActive: boolean;
  /** Opções de formato disponíveis para esta categoria. */
  formats: FormatOption[];
  /** Callback quando uma pétala é selecionada. */
  onSelect: (category: string) => void;
  /** Callback quando um formato é escolhido. */
  onFormatSelect: (format: string) => void;
}

export const RadialPetal: React.FC<RadialPetalProps> = ({
  angle,
  categoryName,
  category,
  icon,
  isActive,
  formats,
  onSelect,
  onFormatSelect,
}) => {
  const handleClick = (): void => {
    onSelect(category);
  };

  const handleFormatClick = (format: string): void => {
    onFormatSelect(format);
  };

  // Calcula a posição da pétala ao redor do centro.
  const radius = 110;
  const rad = (angle * Math.PI) / 180;
  const x = radius * Math.cos(rad);
  const y = radius * Math.sin(rad);

  return (
    <div
      className="petal-layer"
      style={{
        transform: `translate(${x}px, ${y}px) rotate(${angle}deg)`,
      }}
    >
      <button
        type="button"
        className="radial-petal"
        data-active={isActive ? "true" : "false"}
        aria-label={`Categoria ${categoryName}`}
        onClick={handleClick}
        onKeyDown={(e) => {
          if (e.key === "Enter" || e.key === " ") {
            e.preventDefault();
            handleClick();
          }
        }}
      >
        <span className="petal-icon" aria-hidden="true">
          {icon}
        </span>
        <span className="petal-label">{categoryName}</span>
      </button>

      {isActive && formats.length > 0 && (
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
                onClick={() => handleFormatClick(fmt.value)}
              >
                {fmt.value.toUpperCase()}
              </button>
            ))}
          </div>
        </div>
      )}
    </div>
  );
};
