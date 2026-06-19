// Toast notification for displaying errors to the user.
// Auto-dismisses after a timeout, can be manually dismissed.

import { useEffect, useCallback } from "react";
import "./ErrorToast.css";

export interface ErrorToastItem {
  id: number;
  message: string;
}

interface ErrorToastProps {
  toasts: ErrorToastItem[];
  onDismiss: (id: number) => void;
}

export function ErrorToast({ toasts, onDismiss }: ErrorToastProps) {
  if (toasts.length === 0) return null;

  return (
    <div className="error-toast-container">
      {toasts.map((toast) => (
        <ErrorToastEntry key={toast.id} toast={toast} onDismiss={onDismiss} />
      ))}
    </div>
  );
}

function ErrorToastEntry({
  toast,
  onDismiss,
}: {
  toast: ErrorToastItem;
  onDismiss: (id: number) => void;
}) {
  const dismiss = useCallback(() => onDismiss(toast.id), [onDismiss, toast.id]);

  useEffect(() => {
    const timer = setTimeout(dismiss, 6000);
    return () => clearTimeout(timer);
  }, [dismiss]);

  return (
    <div className="error-toast" onClick={dismiss}>
      <span className="error-toast-icon">&#9888;</span>
      <span className="error-toast-message">{toast.message}</span>
      <button className="error-toast-close" onClick={dismiss}>
        &#10005;
      </button>
    </div>
  );
}
