import type { ReactNode } from "react";
import { X } from "lucide-react";

interface ModalProps {
  title: string;
  subtitle?: string;
  children: ReactNode;
  onClose: () => void;
  size?: "normal" | "wide";
}

export function Modal({
  title,
  subtitle,
  children,
  onClose,
  size = "normal",
}: ModalProps) {
  return (
    <div className="modal-backdrop" role="presentation" onMouseDown={onClose}>
      <section
        className={`modal modal-${size}`}
        role="dialog"
        aria-modal="true"
        aria-label={title}
        onMouseDown={(event) => event.stopPropagation()}
      >
        <header className="modal-header">
          <div>
            <h2>{title}</h2>
            {subtitle && <p>{subtitle}</p>}
          </div>
          <button
            type="button"
            className="icon-button"
            aria-label="Close / 閉じる"
            onClick={onClose}
          >
            <X size={18} />
          </button>
        </header>
        {children}
      </section>
    </div>
  );
}
