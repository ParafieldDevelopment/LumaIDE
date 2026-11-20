# Tool Windows

This folder contains dockable IDE panels. Each panel is registered in Main.vue through the
shared tool-window registry and rendered inside a dock region.

## Add a panel

1. Create `YourPanel.vue` here.
2. Accept `workspacePath: string` as a prop when the panel needs workspace context.
3. Wrap the panel in `<LumToolWindow title="...">`, forward its `hide` event, and place content in the default slot.
4. Register it with `registerToolWindow({ id, title, anchor, icon, component, order, hotkey })`.
5. Add the tool to the appropriate stripe and dock renderer in Main.vue.
6. Keep panel state in a composable if it must be shared; avoid panel-specific global state otherwise.

## Standard panel pattern

```vue
<script setup lang="ts">
import LumToolWindow from "../components/LumToolWindow.vue";
defineProps<{ workspacePath: string }>();
const emit = defineEmits<{ (event: "hide"): void }>();
</script>

<template>
  <LumToolWindow title="Panel Name" @hide="emit('hide')">
    <!-- Panel body -->
  </LumToolWindow>
</template>
```

The common `LumToolWindow` provides the header, action slot, and hide control.
