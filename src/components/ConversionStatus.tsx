/**
 * ConversionStatus.tsx
 * Exibe o progresso de uma conversão em andamento.
 * Aparece como um painel flutuante no canto inferior direito.
 */

import React from "react";
import { ConversionState, ConversionStatus as ConversionStatusType } from "@/core/conversion-state";

export interface ConversionStatusProps {
  conversion: ConversionState;
  visible: boolean;
  onCancel: () => void;
}

const STATUS_LABELS: Record<ConversionStatusType, { label: string; icon: string }> = {
  pending: { label: "Preparando...", icon: "⏳" },
  "in-progress": { label: "Convertendo...", icon: "🔄" },
  completed: { label: "Concluído", icon: "✅" },
  failed: { label: "Falha", icon: "❌" },
  cancelled: { label: "Cancelado", icon: "🛑" },
};

export const ConversionStatus: React.FC<ConversionStatusProps> = ({
  conversion,
  visible,
  onCancel,
}) => {
  if (!visible) return null;

  const statusInfo = STATUS_LABELS[conversion.status];
  const progressPct = Math.round(conversion.progress * 100);

  return (
    <div className={`conversion-status visible`} role="status" aria-live="polite">
      <div className="status-header">
        <span className="status-icon" aria-hidden="true">
          {statusInfo.icon}
        </span>
        <div>
          <div className="status-title">{statusInfo.label}</div>
          <div className="status-detail">
            {conversion.sourcePath.split("/").pop() ?? conversion.sourcePath}
          </div>
        </div>
      </div>

      <div className="progress-bar">
        <div
          className={`progress-fill ${
            conversion.status === "completed"
              ? "completed"
              : conversion.status === "failed"
              ? "failed"
              : ""
          }`}
          style={{ width: `${progressPct}%` }}
          role="progressbar"
          aria-valuenow={progressPct}
          aria-valuemin={0}
          aria-valuemax={100}
        />
      </div>

      {conversion.error && (
        <div className="status-detail" style={{ color: "#f87171" }}>
          {conversion.error}
        </div>
      )}

      {conversion.status === "in-progress" && (
        <div className="status-actions">
          <button
            type="button"
            className="btn-cancel"
            onClick={onCancel}
            aria-label="Cancelar conversão"
          >
            Cancelar
          </button>
        </div>
      )}
    </div>
  );
};
