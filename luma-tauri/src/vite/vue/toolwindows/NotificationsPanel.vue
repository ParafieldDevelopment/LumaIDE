<script setup lang="ts">
import { computed } from "vue";
import LumToolWindow from "../components/LumToolWindow.vue";
import { notifications, clearNotifications, dismissNotification } from "../composables/useNotifications";
const emit = defineEmits<{ (event: "hide"): void }>();
const unread = computed(() => notifications.value.filter((n) => !n.read).length);
function markAllRead() { notifications.value.forEach((n) => { n.read = true; }); }
</script>
<template>
  <LumToolWindow title="Notifications" @hide="emit('hide')">
    <div class="lum-notifications">
      <header class="lum-notifications-toolbar">
        <span>{{ unread }} unread</span>
        <button v-if="unread" @click="markAllRead">Mark all read</button>
        <button v-if="notifications.length" @click="clearNotifications">Clear all</button>
      </header>
      <div v-if="!notifications.length" class="lum-notifications-empty">
        <p>You're all caught up.</p><span>IDE events, build results, Git feedback, and warnings will appear here.</span>
      </div>
      <div v-else class="lum-notifications-list">
        <article v-for="item in notifications" :key="item.id" class="lum-notification" :class="{ unread: !item.read, [item.type]: true }" @click="item.read = true">
          <div class="lum-notification-main"><span class="lum-notification-title">{{ item.title }}</span><p>{{ item.message }}</p><small>{{ item.time }}</small></div>
          <button class="lum-notification-dismiss" title="Dismiss" aria-label="Dismiss notification" @click.stop="dismissNotification(item.id)">×</button>
        </article>
      </div>
    </div>
  </LumToolWindow>
</template>
<style scoped>
.lum-notifications { flex:1; min-height:0; overflow:auto; color:var(--lum-fg); font-size:var(--lum-text-sm); }
.lum-notifications-toolbar { display:flex; align-items:center; gap:8px; padding:8px 12px; border-bottom:1px solid var(--lum-border-muted); color:var(--lum-fg-muted); }
.lum-notifications-toolbar button,.lum-notification-dismiss { border:0; background:transparent; color:var(--lum-fg-muted); cursor:pointer; padding:3px 6px; border-radius:var(--lum-radius-sm); }
.lum-notifications-toolbar button:hover,.lum-notification-dismiss:hover { background:var(--lum-bg-hover); color:var(--lum-fg-strong); }
.lum-notifications-empty { display:flex; flex-direction:column; align-items:center; justify-content:center; min-height:160px; text-align:center; gap:8px; padding:24px; color:var(--lum-fg-muted); }
.lum-notifications-empty p { margin:0; color:var(--lum-fg); }
.lum-notification { display:flex; gap:10px; padding:10px 12px; border-bottom:1px solid var(--lum-border-muted); }
.lum-notification.unread { background:var(--lum-bg-hover); }
.lum-notification-main { flex:1; min-width:0; }
.lum-notification-title { font-weight:600; }
.lum-notification p { margin:4px 0; overflow-wrap:anywhere; }
.lum-notification small { color:var(--lum-fg-muted); }
.lum-notification-dismiss { align-self:flex-start; font-size:18px; line-height:1; }
</style>