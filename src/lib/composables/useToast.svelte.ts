/**
 * Toast notification composable
 */

export type ToastType = "success" | "error" | "info" | "warning";

export interface Toast {
  id: number;
  message: string;
  type: ToastType;
}

let nextId = 0;

export function useToast() {
  let toasts: Toast[] = $state([]);

  function show(message: string, type: ToastType = "success", duration = 3000) {
    const id = nextId++;
    toasts = [...toasts, { id, message, type }];

    if (duration > 0) {
      setTimeout(() => {
        dismiss(id);
      }, duration);
    }
  }

  function dismiss(id: number) {
    toasts = toasts.filter((t) => t.id !== id);
  }

  function success(message: string, duration = 3000) {
    show(message, "success", duration);
  }

  function error(message: string, duration = 5000) {
    show(message, "error", duration);
  }

  function info(message: string, duration = 3000) {
    show(message, "info", duration);
  }

  function warning(message: string, duration = 4000) {
    show(message, "warning", duration);
  }

  return {
    get toasts() {
      return toasts;
    },
    show,
    dismiss,
    success,
    error,
    info,
    warning,
  };
}
