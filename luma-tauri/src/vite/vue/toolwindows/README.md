# Tool Windows

This folder contains dockable IDE panels. Panels are registered in `Main.vue` through the
shared tool-window registry and rendered in the left or right dock.

## Dock positions

Tool buttons live on the vertical edge stripes, in four independent groups:

- `left-top`
- `left-bottom`
- `right-top`
- `right-bottom`

The button position is saved per workspace in `.luma`. Drag a tool button into another
corner group to move it. A panel opens on the corresponding left or right side, and both
side docks can be visible at the same time. There is no separate horizontal bottom toolbar.

## Add a panel

1. Create `YourPanel.vue` here.
2. Accept `workspacePath: string` as a prop when the panel needs workspace context.
3. Wrap the panel in `<LumToolWindow title="...">`, forward its `hide` event, and put the
   panel body in the default slot.
4. Register it in `Main.vue` with `registerToolWindow({ id, title, position, icon, component })`.
5. Place the button in the appropriate corner group through the shared registry.
6. Keep persistent panel state in a composable when it must survive hiding/unmounting.

## Terminal sessions

The terminal panel supports multiple shell tabs. Hiding the terminal dock detaches its
xterm views but leaves backend shell sessions alive. Reopening the dock restores sessions
and their retained output. Closing an individual terminal tab terminates that shell;
closing LumaIDE terminates all remaining terminal sessions.

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

The common `LumToolWindow` provides a single header hide control and an optional
`actions` slot. Keep panel-specific actions in that slot rather than adding duplicate
outer close buttons.
