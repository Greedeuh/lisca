import { createContext, useContext, useState, useCallback, useRef } from "react";
import { ErrorToast, type ErrorToastItem } from "../components/common";

interface ToastContextValue {
  addToast: (message: string) => void;
}

const ToastContext = createContext<ToastContextValue>({ addToast: () => {} });

export function useToast() {
  return useContext(ToastContext);
}

export function ToastProvider({ children }: { children: React.ReactNode }) {
  const [toasts, setToasts] = useState<ErrorToastItem[]>([]);
  const toastIdRef = useRef(0);

  const addToast = useCallback((message: string) => {
    const id = ++toastIdRef.current;
    setToasts((prev) => [...prev, { id, message }]);
  }, []);

  const dismissToast = useCallback((id: number) => {
    setToasts((prev) => prev.filter((t) => t.id !== id));
  }, []);

  return (
    <ToastContext.Provider value={{ addToast }}>
      {children}
      <ErrorToast toasts={toasts} onDismiss={dismissToast} />
    </ToastContext.Provider>
  );
}
