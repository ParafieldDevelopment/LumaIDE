<script setup lang="ts">
import { computed } from "vue";
import { useModals, type ModalAction, type ModalItem } from "~/vue/components/modals";
import LumButton from "./LumButton.vue";
import LumModal from "./LumModal.vue";

const { modals, dismiss } = useModals();
const activeModal = computed(() => modals.value[0]);

function onOpenChange(id: number, open: boolean) {
  if (!open) dismiss(id);
}

function runAction(item: ModalItem, action: ModalAction) {
  const close = () => dismiss(item.id);
  action.onClick?.(item, close);

  if (action.close !== false) close();
}
</script>

<template>
  <LumModal
      v-if="activeModal"
      :key="activeModal.id"
      :open="true"
      :size="activeModal.size"
      :persistent="activeModal.persistent"
      @update:open="onOpenChange(activeModal.id, $event)"
  >
    <template #title>{{ activeModal.title }}</template>
    {{ activeModal.body }}

    <template v-if="activeModal.actions?.length" #footer>
      <LumButton
          v-for="(action, index) in activeModal.actions"
          :key="`${activeModal.id}-${index}`"
          :variant="action.variant"
          @click="runAction(activeModal, action)"
      >
        {{ action.label }}
      </LumButton>
    </template>
  </LumModal>
</template>
