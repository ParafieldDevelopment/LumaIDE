import { markRaw, ref, type Component } from "vue";

export interface ModalAction {
  label: string;
  variant?: "default" | "primary" | "danger" | "ghost" | "link";
  close?: boolean;
  onClick?: (item: ModalItem, close: () => void) => void;
}

export interface ModalOptions {
  size?: "sm" | "md" | "lg" | "xl";
  persistent?: boolean;
  closeOnBackdrop?: boolean;
  showCloseButton?: boolean;
  actions?: ModalAction[];
  componentProps?: Record<string, unknown>;
}

export interface ModalItem extends ModalOptions {
  id: number;
  title?: string;
  component: Component;
}

const modals = ref<ModalItem[]>([]);
let nextId = 1;

export function useModals() {
  function modal(title: string, component: Component, options?: ModalOptions): ModalItem;
  function modal(component: Component, options?: ModalOptions): ModalItem;
  function modal(
    titleOrComponent: string | Component,
    componentOrOptions?: Component | ModalOptions,
    options: ModalOptions = {},
  ): ModalItem {
    const hasTitle = typeof titleOrComponent === "string";
    const title = hasTitle ? titleOrComponent : undefined;
    const component = hasTitle ? componentOrOptions as Component : titleOrComponent;
    const modalOptions = hasTitle
      ? options
      : (componentOrOptions as ModalOptions | undefined) ?? {};
    const item: ModalItem = { id: nextId++, title, component: markRaw(component), ...modalOptions };

    modals.value.push(item);
    return item;
  }

  function dismiss(id: number) {
    modals.value = modals.value.filter((item) => item.id !== id);
  }

  return { modals, modal, dismiss };
}

export function modal(title: string, component: Component, options?: ModalOptions): ModalItem;
export function modal(component: Component, options?: ModalOptions): ModalItem;
export function modal(
  titleOrComponent: string | Component,
  componentOrOptions?: Component | ModalOptions,
  options?: ModalOptions,
): ModalItem {
  return typeof titleOrComponent === "string"
    ? useModals().modal(titleOrComponent, componentOrOptions as Component, options)
    : useModals().modal(titleOrComponent, (componentOrOptions as ModalOptions | undefined));
}
