import { ref } from "vue";

export interface ModalAction {
  label: string;
  variant?: "default" | "primary" | "danger" | "ghost" | "link";
  close?: boolean;
  onClick?: (item: ModalItem, close: () => void) => void;
}

export interface ModalOptions {
  size?: "sm" | "md" | "lg" | "xl";
  persistent?: boolean;
  actions?: ModalAction[];
}

export interface ModalItem extends ModalOptions {
  id: number;
  title: string;
  body: string;
}

const modals = ref<ModalItem[]>([]);
let nextId = 1;

export function useModals() {
  function modal(title: string, body = "", options: ModalOptions = {}) {
    const item: ModalItem = { id: nextId++, title, body, ...options };

    modals.value.push(item);
    return item;
  }

  function dismiss(id: number) {
    modals.value = modals.value.filter((item) => item.id !== id);
  }

  return { modals, modal, dismiss };
}

export function modal(title: string, body = "", options: ModalOptions = {}) {
  return useModals().modal(title, body, options);
}
