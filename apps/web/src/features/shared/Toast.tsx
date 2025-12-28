export type ToastMessage = {
  message: string;
  tone?: "error" | "info";
};

type ToastProps = {
  toast: ToastMessage;
  onDismiss: () => void;
};

export function Toast({ toast, onDismiss }: ToastProps) {
  return (
    <div
      className={`toast ${toast.tone === "error" ? "toast-error" : "toast-info"}`}
      role="status"
      aria-live="polite"
    >
      <span>{toast.message}</span>
      <button
        className="icon-button toast-close"
        type="button"
        onClick={onDismiss}
        aria-label="Dismiss notification"
      >
        ×
      </button>
    </div>
  );
}
