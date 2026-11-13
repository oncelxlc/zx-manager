import type { ReactNode } from "react";
import { toast, Toaster } from "../components/ui/toast";

export function notify(message: string, error = false) {
  toast.add({ title: message, type: error ? "error" : "success", priority: error ? "high" : "low" });
}

export function ToastProvider({ children }: { children: ReactNode }) {
  return <Toaster limit={2}>{children}</Toaster>;
}
