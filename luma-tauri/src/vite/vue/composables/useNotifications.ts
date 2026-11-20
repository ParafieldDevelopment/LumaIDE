import { ref } from "vue";
export type NotificationType = "info" | "success" | "warning" | "error";
export interface LumNotification { id: string; title: string; message: string; type: NotificationType; time: string; read: boolean; }
export const notifications = ref<LumNotification[]>([]);
let nextId = 1;
export function pushNotification(title: string, message: string, type: NotificationType = "info") {
  notifications.value.unshift({ id: String(nextId++), title, message, type, time: new Date().toLocaleTimeString(), read: false });
}
export function dismissNotification(id: string) { notifications.value = notifications.value.filter((item) => item.id !== id); }
export function clearNotifications() { notifications.value = []; }
