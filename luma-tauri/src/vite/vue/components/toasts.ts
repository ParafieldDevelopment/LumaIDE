import { ref } from "vue";

export interface ToastItem {
  id: number;
  kind: "success" | "info" | "warning" | "danger";
  title: string;
  body: string;
}

const toasts = ref<ToastItem[]>([]);
let nextId = 1;

export function useToasts() {
  function toast(kind: ToastItem["kind"], title: string, body = "", duration = 3500) {
    const item: ToastItem = { id: nextId++, kind, title, body };

    toasts.value.push(item);
    window.setTimeout(() => dismiss(item.id), duration);

    return item;
  }

  function dismiss(id: number) {
    toasts.value = toasts.value.filter((t) => t.id !== id);
  }

  return { toasts, toast, dismiss };
}

export function toast(kind: ToastItem["kind"], title: string, body = "", duration = 3500) {
  return useToasts().toast(kind, title, body, duration);
}