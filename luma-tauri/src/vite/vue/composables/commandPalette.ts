/** Shared open state for the IDE command palette, so any action can open it. */
import { ref } from "vue";

export const commandPaletteOpen = ref(false);

export function openCommandPalette(): void {
  commandPaletteOpen.value = true;
}

export function closeCommandPalette(): void {
  commandPaletteOpen.value = false;
}
