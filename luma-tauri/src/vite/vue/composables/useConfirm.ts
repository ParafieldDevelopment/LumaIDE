/**
 * Promise-based confirm dialogs built on the shared modal stack, plus the two
 * IDE prompts (exit, open project) and their "Don't ask again" preferences.
 * Preferences live in localStorage; call `resetDialogPrefs()` to bring the prompts back.
 */
import { reactive, watch } from "vue";
import { useModals } from "~/vue/components/modals";
import ModalConfirmBody from "~/vue/components/ModalConfirmBody.vue";

export interface ConfirmButton {
  label: string;
  value: string;
  variant?: "default" | "primary" | "danger";
}

export interface ConfirmOptions {
  title: string;
  message: string;
  checkboxLabel?: string;
  /** First button is the default (focused) one. */
  buttons: ConfirmButton[];
}

export interface ConfirmResult {
  /** The clicked button's value, or null if the dialog was closed (X / Esc). */
  value: string | null;
  dontAsk: boolean;
}

export function confirmDialog(options: ConfirmOptions): Promise<ConfirmResult> {
  const { modal, modals } = useModals();

  return new Promise((resolve) => {
    const state = reactive({ dontAsk: false });
    let settled = false;
    let stop: (() => void) | undefined;

    const finish = (value: string | null) => {
      if (settled) return;
      settled = true;
      stop?.();
      resolve({ value, dontAsk: state.dontAsk });
    };

    const item = modal(options.title, ModalConfirmBody, {
      size: "sm",
      componentProps: { message: options.message, checkboxLabel: options.checkboxLabel, state },
      actions: options.buttons.map((button) => ({
        label: button.label,
        variant: button.variant,
        onClick: () => finish(button.value),
      })),
    });

    // Closing with the X button or Escape removes the modal without running an action.
    stop = watch(modals, (list) => {
      if (!list.some((entry) => entry.id === item.id)) finish(null);
    });
  });
}

const EXIT_KEY = "luma.confirmExit";
const OPEN_KEY = "luma.openProjectMode";

function readPref(key: string): string | null {
  try {
    return localStorage.getItem(key);
  } catch {
    return null;
  }
}

function writePref(key: string, value: string) {
  try {
    localStorage.setItem(key, value);
  } catch {
    // Preference just won't persist.
  }
}

export function resetDialogPrefs() {
  try {
    localStorage.removeItem(EXIT_KEY);
    localStorage.removeItem(OPEN_KEY);
  } catch {
    // Nothing to reset.
  }
}

/** Ask before closing the IDE window. Resolves true if it is OK to exit. */
export async function confirmExit(): Promise<boolean> {
  if (readPref(EXIT_KEY) === "never") return true;
  const result = await confirmDialog({
    title: "Confirm Exit",
    message: "Are you sure you want to exit?",
    checkboxLabel: "Don't ask again",
    buttons: [
      { label: "Exit", value: "exit", variant: "primary" },
      { label: "Cancel", value: "cancel" },
    ],
  });
  if (result.value !== "exit") return false;
  if (result.dontAsk) writePref(EXIT_KEY, "never");
  return true;
}

export type OpenMode = "this" | "new" | "cancel";

/** Ask where a different project should open. */
export async function askOpenMode(projectTitle: string): Promise<OpenMode> {
  const saved = readPref(OPEN_KEY);
  if (saved === "this" || saved === "new") return saved;

  const result = await confirmDialog({
    title: "Open Project",
    message: `Where would you like to open the project '${projectTitle}'?`,
    checkboxLabel: "Don't ask again",
    buttons: [
      { label: "This Window", value: "this", variant: "primary" },
      { label: "New Window", value: "new" },
      { label: "Cancel", value: "cancel" },
    ],
  });
  if (result.value === "this" || result.value === "new") {
    if (result.dontAsk) writePref(OPEN_KEY, result.value);
    return result.value;
  }
  return "cancel";
}
