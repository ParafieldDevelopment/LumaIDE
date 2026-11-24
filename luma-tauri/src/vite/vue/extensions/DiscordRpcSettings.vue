<script setup lang="ts">
import { onMounted, ref, watch } from "vue";
import { LumCard, LumField, LumSwitch, LumBadge } from "~/vue/components";
import { invoke } from "@tauri-apps/api/core";
import { toast } from "~/vue/components/toasts";

interface DiscordRpcConfig {
  enabled: boolean;
  show_workspace: boolean;
  show_project_type: boolean;
  show_git_branch: boolean;
}

const loading = ref(true);

const config = ref<DiscordRpcConfig>({
  enabled: true,
  show_workspace: true,
  show_project_type: true,
  show_git_branch: true,
});

onMounted(async () => {
  try {
    const loaded = await invoke<DiscordRpcConfig>("get_discord_rpc_config");
    config.value = {
      enabled: loaded.enabled ?? true,
      show_workspace: loaded.show_workspace ?? true,
      show_project_type: loaded.show_project_type ?? true,
      show_git_branch: loaded.show_git_branch ?? true,
    };
  } catch (error) {
    console.error("Failed to load Discord Rich Presence configuration:", error);
  } finally {
    loading.value = false;
  }
});

async function saveConfig() {
  if (loading.value) return;
  try {
    await invoke("set_discord_rpc_config", { config: config.value });
  } catch (error) {
    console.error("Failed to save Discord Rich Presence configuration:", error);
    toast("danger", "Failed to save settings", String(error));
  }
}

watch(config, saveConfig, { deep: true });
</script>

<template>
  <div class="rpc-settings-group">
    <LumCard class="settings-card">
      <template #header>
        <div class="card-header">
          <div class="card-title-group">
            <p class="card-title">Discord Rich Presence</p>
            <LumBadge :variant="config.enabled ? 'success' : 'neutral'">
              {{ config.enabled ? "Enabled" : "Disabled" }}
            </LumBadge>
          </div>
        </div>
      </template>

      <div class="card-body">
        <LumField
          label="Enable Discord Rich Presence"
          help="Broadcasts your LumaIDE status to your Discord profile."
        >
          <LumSwitch v-model="config.enabled" />
        </LumField>

        <div v-if="config.enabled" class="sub-settings">
          <LumField
            label="Show Active Workspace"
            help="Displays the name of the project you are working on."
          >
            <LumSwitch v-model="config.show_workspace" />
          </LumField>

          <LumField
            label="Show Git Branch"
            help="Displays your active git branch when inside a git repository."
          >
            <LumSwitch v-model="config.show_git_branch" />
          </LumField>

          <LumField
            label="Show Project Type"
            help="Displays project environment details (such as Lua, Luau, or Roblox)."
          >
            <LumSwitch v-model="config.show_project_type" />
          </LumField>
        </div>
      </div>
    </LumCard>

    <LumCard class="settings-card info-card">
      <template #header>
        <p class="card-title">Presence Behavior</p>
      </template>
      <div class="card-body info-text">
        <p>
          When in the <strong>Project Selection</strong> screen (or when no IDE editor is open), your Discord presence simply shows <strong>Idle</strong> with the Luma icon.
        </p>
        <p>
          Once you open a project, your status automatically shows what project and file you are editing.
        </p>
      </div>
    </LumCard>
  </div>
</template>

<style scoped lang="scss">
.rpc-settings-group {
  display: flex;
  flex-direction: column;
  gap: 16px;
  width: 100%;
}

.settings-card {
  width: 100%;
}

.card-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  width: 100%;
}

.card-title-group {
  display: flex;
  align-items: center;
  gap: 12px;
}

.card-title {
  font-size: 15px;
  font-weight: 600;
  color: var(--lum-gray-100);
  margin: 0;
}

.card-body {
  display: flex;
  flex-direction: column;
  gap: 16px;
  padding-top: 8px;
}

.sub-settings {
  display: flex;
  flex-direction: column;
  gap: 14px;
  padding-left: 12px;
  border-left: 2px solid var(--lum-border);
  margin-top: 4px;
}

.info-card {
  border-color: color-mix(in srgb, var(--lum-border), transparent 30%);
}

.info-text {
  font-size: 13px;
  line-height: 1.6;
  color: var(--lum-gray-300);

  p {
    margin: 0 0 8px 0;

    &:last-child {
      margin-bottom: 0;
    }
  }
}
</style>
