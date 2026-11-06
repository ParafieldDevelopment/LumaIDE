<script setup lang="ts">
/**
 * ComponentShowcase.vue — sanity check for every component in
 * vue/components/. Each section maps to components in the library.
 */
import {
  LumAccordion,
  LumAccordionItem,
  LumAlert,
  LumAvatar,
  LumBreadcrumbs,
  LumBadge,
  LumButton,
  LumCard,
  LumCheckbox,
  LumChip,
  LumCodeBlock,
  LumCommandPalette,
  LumDivider,
  LumField,
  LumFileTabs,
  LumIconBtn,
  LumInput,
  LumKbd,
  LumList,
  LumListItem,
  LumMenu,
  LumMenuItem,
  LumModal,
  LumNumber,
  LumPagination,
  LumProgress,
  LumRadio,
  LumSelect,
  LumSkeleton,
  LumSlider,
  LumSpinner,
  LumSidenav,
  LumSidenavItem,
  LumSwitch,
  LumTabs,
  LumTable,
  LumTextarea,
  LumToasts,
  LumTooltip,
  LumTree,
  toast,
  type TreeNode,
} from "./components";
import { computed, reactive, ref } from "vue";

/* ---------------- theme override demo (what a theme/extension does) ---------------- */

const altTheme = ref(false);
const themeVars = computed(() =>
  altTheme.value
    ? ({
        "--lum-accent": "#e07f2e",
        "--lum-accent-hover": "#f59e4c",
        "--lum-accent-active": "#c8661d",
        "--lum-accent-fg": "#1a1105",
        "--lum-accent-muted": "rgba(224, 127, 46, 0.16)",
        "--lum-focus-ring": "#e07f2e",
        "--lum-button-border-radius": "999px",
        "--lum-switch-bg-checked": "#e07f2e",
        "--lum-checkbox-bg-checked": "#e07f2e",
        "--lum-radio-bg-checked": "#e07f2e",
        "--lum-slider-bg-filled": "#e07f2e",
      } as Record<string, string>)
    : {},
);

/* ---------------- demo state ---------------- */

const textValue = ref("Hello Luma");
const invalidValue = ref("");
const textareaValue = ref("Multi-line\ntext buffer…");

const checkedA = ref(true);
const checkedIndeterminate = ref(true);
const radioPick = ref("vue");
const switchOn = ref(true);
const agrees = ref(false);

const sliderValue = ref(40);
const selectValue = ref("b");
const numberValue = ref(4);

const progressValue = ref(35);
const progressRunning = ref(false);
let progressTimer: ReturnType<typeof setTimeout> | undefined;
const toggleProgress = () => {
  if (progressRunning.value) {
    progressRunning.value = false;
    if (progressTimer) clearInterval(progressTimer);
    progressTimer = undefined;
    return;
  }
  progressRunning.value = true;
  progressTimer = setInterval(() => (progressValue.value = (progressValue.value + 1) % 101), 80);
};

const chips = ref(["typescript", "rust", "tauri", "vue"]);

const modalOpen = ref(false);
const paletteOpen = ref(false);
const menuOpen = ref(false);

const activeTab = ref(1);
const tabs = [
  { label: "Editor" },
  { label: "Source Control" },
  { label: "Run" },
  { label: "Extensions" },
];

const fileTabs = ref([
  { id: 1, name: "main.rs", dirty: true },
  { id: 2, name: "style.scss" },
  { id: 3, name: "App.vue" },
]);
const activeFile = ref(1);

const treeNodes: TreeNode[] = [
  {
    label: "src",
    children: [
      { label: "main.ts" },
      {
        label: "styles",
        children: [{ label: "style.scss" }, { label: "uicomponents.scss", ignored: true }],
      },
    ],
  },
  { label: "package-lock.json", cut: true },
];
const treeSelected = ref<TreeNode | null>(null);

const accordionOpen = reactive([true, false, false]);

const page = ref(3);
const selectedList = ref(1);
const commands = [
  { id: 1, label: "Open file…", shortcut: "Ctrl+O" },
  { id: 2, label: "Save workspace", shortcut: "Ctrl+S" },
  { id: 3, label: "Toggle theme", shortcut: "Ctrl+K" },
  { id: 4, label: "Build project", shortcut: "Ctrl+Shift+B" },
];
</script>

<template>
  <div class="showcase" :style="themeVars">
    <!-- header -->
    <header class="showcase-header">
      <div>
        <h1>LumaUI Showcase</h1>
        <p class="lum-text-muted">
          Every component from <code class="lum-code">vue/components/</code>, styled by
          <code class="lum-code">styles/uicomponents.scss</code>.
        </p>
      </div>
      <label class="lum-checkbox-row">
        <LumCheckbox v-model="altTheme" aria-label="Apply mock theme" />
        <span>Apply mock theme override</span>
      </label>
    </header>

    <!-- buttons -->
    <section class="showcase-section">
      <h2>Buttons</h2>

      <div class="showcase-row">
        <LumButton>Secondary</LumButton>
        <LumButton variant="primary">Primary</LumButton>
        <LumButton variant="danger">Danger</LumButton>
        <LumButton variant="ghost">Ghost</LumButton>
        <LumButton variant="link">Link</LumButton>
        <LumButton disabled>Disabled</LumButton>
        <LumButton variant="primary" loading data-tip="Loading via prop">Loading</LumButton>
      </div>

      <div class="showcase-row">
        <LumButton size="sm">Small</LumButton>
        <LumButton>Default</LumButton>
        <LumButton size="lg">Large</LumButton>
        <LumButton variant="primary" icon block>
          <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 5v14M5 12h14" /></svg>
          Block button
        </LumButton>
      </div>

      <div class="showcase-row">
        <LumIconBtn tip="Settings" aria-label="Settings">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 1 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 1 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.01a1.65 1.65 0 0 0 1-1.51V3a2 2 0 1 1 4 0v.09a1.65 1.65 0 0 0 1 1.51h.01a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.01a1.65 1.65 0 0 0 1.51 1H21a2 2 0 1 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" /></svg>
        </LumIconBtn>
        <span>Focus anything with <LumKbd keys="Tab" /> to see focus rings</span>
      </div>
    </section>

    <!-- inputs -->
    <section class="showcase-section">
      <h2>Inputs</h2>

      <div class="lum-form-lg">
        <div class="lum-form-row">
          <LumField label="Default input" :help="`Value: ${textValue}`">
            <LumInput v-model="textValue" placeholder="Type here…" />
          </LumField>

          <LumField label="Invalid input">
            <LumInput v-model="invalidValue" invalid placeholder="Required" />
            <template #help><span>This field is required</span></template>
          </LumField>
        </div>

        <div class="lum-form-row">
          <LumField label="Small">
            <LumInput v-model="textValue" size="sm" placeholder="Small" />
          </LumField>
          <LumField label="Large + icon">
            <LumInput v-model="textValue" size="lg" placeholder="Search…">
              <template #icon>
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8" /><path d="m21 21-4.3-4.3" /></svg>
              </template>
            </LumInput>
          </LumField>
          <LumField label="Disabled">
            <LumInput v-model="textValue" disabled placeholder="Can't type here" />
          </LumField>
        </div>

        <div class="lum-form-row">
          <LumField label="Select">
            <LumSelect v-model="selectValue" :options="[
              { value: 'a', label: 'Option A' },
              { value: 'b', label: 'Option B' },
              { value: 'c', label: 'Option C' },
              { value: 'd', label: 'Disabled option', disabled: true },
            ]" />
          </LumField>

          <LumField label="Number stepper">
            <LumNumber v-model="numberValue" :min="0" :max="10" />
          </LumField>

          <LumField label="Slider">
            <div class="lum-slider-group">
              <LumSlider v-model="sliderValue" />
              <span class="lum-slider-value">{{ sliderValue }}%</span>
            </div>
          </LumField>
        </div>

        <LumField label="Textarea">
          <LumTextarea v-model="textareaValue" rows="3" />
        </LumField>
      </div>
    </section>

    <!-- checkbox / radio / switch -->
    <section class="showcase-section">
      <h2>Checkbox · Radio · Switch</h2>

      <div class="showcase-row">
        <label class="lum-checkbox-row">
          <LumCheckbox v-model="checkedA" />
          <span>Checked? {{ checkedA }}</span>
        </label>
        <label class="lum-checkbox-row">
          <LumCheckbox v-model="checkedIndeterminate" indeterminate />
          <span>Indeterminate</span>
        </label>
        <label class="lum-checkbox-row">
          <LumCheckbox v-model="agrees" disabled />
          <span>Disabled</span>
        </label>
      </div>

      <div class="showcase-row">
        <div class="lum-radio-group horizontal">
          <label class="lum-radio-row">
            <LumRadio v-model="radioPick" value="vue" /><span>Vue</span>
          </label>
          <label class="lum-radio-row">
            <LumRadio v-model="radioPick" value="rust" /><span>Rust</span>
          </label>
          <label class="lum-radio-row">
            <LumRadio v-model="radioPick" value="both" disabled /><span>Both (disabled)</span>
          </label>
        </div>
      </div>

      <div class="showcase-row">
        <LumSwitch v-model="switchOn" />
        <span>Switch is {{ switchOn ? "on" : "off" }}</span>
        <LumSwitch disabled />
      </div>
    </section>

    <!-- loading -->
    <section class="showcase-section">
      <h2>Loading</h2>

      <div class="lum-field" style="margin-bottom: 12px">
        <div class="lum-form-row">
          <LumButton @click="toggleProgress">{{ progressRunning ? "Stop" : "Start" }} progress</LumButton>
          <span class="lum-help">{{ progressValue }}%</span>
        </div>
        <LumProgress :value="progressValue" />
      </div>

      <div style="display: grid; gap: 10px; max-width: 500px; margin-bottom: 16px">
        <LumProgress indeterminate />
        <LumProgress :value="70" striped color="success" />
      </div>

      <div class="showcase-row">
        <LumSpinner />
        <LumSpinner size="lg" color="success" />
        <LumSpinner size="lg" color="warning" />
        <LumSpinner size="lg" color="danger" />
        <span class="lum-dots"><i></i></span>
      </div>

      <div class="showcase-row">
        <LumSkeleton shape="circle" style="width: 36px; height: 36px" />
        <LumSkeleton shape="rect" style="width: 140px; height: 64px" />
        <span style="flex: 1; display: grid">
          <LumSkeleton style="margin: 0" />
          <LumSkeleton style="margin: 0; width: 70%" />
        </span>
      </div>
    </section>

    <!-- badges / chips / avatars -->
    <section class="showcase-section">
      <h2>Badges · Chips · Avatars</h2>

      <div class="showcase-row">
        <LumBadge>neutral</LumBadge>
        <LumBadge variant="primary">primary</LumBadge>
        <LumBadge variant="soft">soft</LumBadge>
        <LumBadge variant="success">stable</LumBadge>
        <LumBadge variant="warning">beta</LumBadge>
        <LumBadge variant="danger">failed</LumBadge>
        <LumBadge variant="info">info</LumBadge>
        <LumBadge dot="online" />
        <LumBadge dot="busy" />
      </div>

      <div class="showcase-row">
        <LumChip v-for="chip in chips" :key="chip" :label="chip" removable @remove="chips = chips.filter((c) => c !== chip)" />
        <span v-if="!chips.length" class="lum-caption">All chips removed</span>
      </div>

      <div class="showcase-row">
        <LumAvatar size="sm">A</LumAvatar>
        <LumAvatar>L</LumAvatar>
        <LumAvatar size="lg" square status="online"><img alt="avatar" src="../public/Logo.svg" /></LumAvatar>
        <LumAvatar size="lg" status="idle">Z</LumAvatar>
        <span class="lum-avatar-stack">
          <LumAvatar>M</LumAvatar>
          <LumAvatar>N</LumAvatar>
          <LumAvatar>O</LumAvatar>
        </span>
      </div>
    </section>

    <!-- cards -->
    <section class="showcase-section">
      <h2>Cards</h2>

      <div class="lum-form-row" style="align-items: stretch">
        <LumCard interactive>
          <template #header>Interactive card</template>
          <p class="lum-text-muted lum-caption">Click-friendly card with hover elevation.</p>
        </LumCard>

        <LumCard elevated>
          <template #header>Elevated card</template>
          <p>Cards compose headers, bodies and footers.</p>
          <template #footer>
            <LumButton size="sm">Action</LumButton>
          </template>
        </LumCard>
      </div>
    </section>

    <!-- modal / menu / palette / tooltip -->
    <section class="showcase-section">
      <h2>Overlays</h2>

      <div class="showcase-row">
        <LumButton variant="primary" @click="modalOpen = true">Open modal</LumButton>
        <LumButton @click="paletteOpen = true">Command palette</LumButton>
        <LumTooltip data-tip="Tooltip with data-tip attribute">
          <LumButton variant="ghost">Hover for tooltip</LumButton>
        </LumTooltip>

        <div class="relative-demo">
          <LumButton @click="menuOpen = !menuOpen">Dropdown menu</LumButton>
          <LumMenu v-if="menuOpen" class="absolute-demo" @mouseleave="menuOpen = false">
            <template #label>Actions</template>
            <LumMenuItem shortcut="Ctrl+O" @click="menuOpen = false">
              Open file
              <template #icon><svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" /><polyline points="7 10 12 15 17 10" /></svg></template>
            </LumMenuItem>
            <LumMenuItem shortcut="Ctrl+S" @click="menuOpen = false">Save</LumMenuItem>
            <LumMenuItem danger @click="menuOpen = false">Delete</LumMenuItem>
          </LumMenu>
        </div>
      </div>

      <LumModal v-model:open="modalOpen">
        <template #title>Example modal</template>
        Dialog with overlay backdrop, header, scrollable body and footer actions.
        <template #footer>
          <LumButton @click="modalOpen = false">Cancel</LumButton>
          <LumButton variant="primary" @click="modalOpen = false; toast('success', 'Confirmed', 'Modal action completed.')">Confirm</LumButton>
        </template>
      </LumModal>

      <LumCommandPalette v-model:open="paletteOpen" :commands="commands" @run="(id) => toast('info', 'Command', `Ran command #${id}`)" />
    </section>

    <!-- tabs -->
    <section class="showcase-section">
      <h2>Tabs</h2>

      <LumTabs v-model="activeTab" :items="[...tabs, { label: 'Disabled', disabled: true }]" />

      <LumFileTabs v-model="activeFile" :files="fileTabs" @close="fileTabs = fileTabs.filter((f) => f.id !== $event)">
        <template #extra><span class="lum-caption">click × to close</span></template>
      </LumFileTabs>

      <LumToolbar title="toolbar with title + separators" style="margin-top: 16px">
        <span class="lum-toolbar-separator"></span>
        <LumIconBtn aria-label="Item">☰</LumIconBtn>
        <span class="lum-toolbar-separator"></span>
        <LumButton size="sm">Run</LumButton>
      </LumToolbar>
    </section>

    <!-- navigation -->
    <section class="showcase-section showcase-columns">
      <div style="flex: 1; min-width: 220px">
        <h2>Sidenav</h2>
        <LumSidenav style="background: var(--lum-bg-base); border-radius: 10px">
          <LumSidenavItem active badge="12">
            Explorer
          </LumSidenavItem>
          <LumSidenavItem>Source Control</LumSidenavItem>
          <LumSidenavItem badge="ok">Run &amp; Debug</LumSidenavItem>
          <LumSidenavItem>Extensions</LumSidenavItem>
        </LumSidenav>

        <h2 style="margin-top: 24px">Breadcrumbs</h2>
        <LumBreadcrumbs :items="['LumaIDE', 'src', 'vite', 'styles', 'uicomponents.scss']" />

        <h2 style="margin-top: 24px">Pagination</h2>
        <LumPagination v-model="page" :pages="7" />
      </div>

      <div style="flex: 1; min-width: 240px">
        <h2>List</h2>
        <LumList style="background: var(--lum-bg-base); border-radius: 10px">
          <LumListItem :selected="selectedList === 1" @select="selectedList = 1">
            src/components
            <template #meta>12</template>
          </LumListItem>
          <LumListItem :selected="selectedList === 2" @select="selectedList = 2">src/styles</LumListItem>
          <LumListItem :selected="selectedList === 3" @select="selectedList = 3">src/vue</LumListItem>
          <LumListItem :selected="selectedList === 4" disabled>package.json</LumListItem>
        </LumList>
      </div>

      <div style="flex: 1; min-width: 260px">
        <h2>Tree</h2>
        <LumTree
          :nodes="treeNodes"
          :selected="treeSelected"
          style="background: var(--lum-bg-base); border-radius: 10px"
          @select="treeSelected = $event"
        />
      </div>
    </section>

    <!-- table -->
    <section class="showcase-section">
      <h2>Table</h2>
      <LumTable>
        <thead>
          <tr>
            <th data-sortable data-sorted="asc">Name</th>
            <th data-sortable>Kind</th>
            <th data-sortable>Status</th>
          </tr>
        </thead>
        <tbody>
          <tr><td>uicomponents.scss</td><td>SCSS</td><td><LumBadge variant="success">ready</LumBadge></td></tr>
          <tr class="selected"><td>style.scss</td><td>SCSS</td><td><LumBadge variant="warning">edited</LumBadge></td></tr>
          <tr><td>Showcase.vue</td><td>Vue</td><td><LumBadge variant="primary">new</LumBadge></td></tr>
          <tr><td>vite.config.ts</td><td>TS</td><td><LumBadge variant="danger">error</LumBadge></td></tr>
        </tbody>
      </LumTable>
    </section>

    <!-- accordion + alerts -->
    <section class="showcase-section showcase-columns">
      <div style="flex: 1; min-width: 240px">
        <h2>Accordion</h2>
        <LumAccordion>
          <LumAccordionItem v-model="accordionOpen[0]" title="Details">Collapsible content.</LumAccordionItem>
          <LumAccordionItem v-model="accordionOpen[1]" title="Changelog">Version history.</LumAccordionItem>
          <LumAccordionItem v-model="accordionOpen[2]" title="License">MIT.</LumAccordionItem>
        </LumAccordion>
      </div>

      <div style="flex: 1.2; min-width: 280px">
        <h2>Alerts</h2>
        <div style="display: grid; gap: 12px">
          <LumAlert kind="info" title="Heads up">Some informative message.</LumAlert>
          <LumAlert kind="success" title="Success" dismissible @close="toast('success', 'Dismissed')">Operation completed.</LumAlert>
          <LumAlert kind="warning" title="Warning">Something needs attention.</LumAlert>
          <LumAlert kind="danger" title="Error">Something went wrong.</LumAlert>
        </div>
      </div>
    </section>

    <!-- dividers / misc -->
    <section class="showcase-section">
      <h2>Dividers &amp; code</h2>

      <LumDivider label="section label divider" />
      <div class="lum-form-row">
        <LumDivider vertical style="min-height: 50px" />
        <span class="lum-caption">vertical divider sits between content</span>
        <LumDivider vertical style="min-height: 50px" />
      </div>
      <LumDivider dashed />

      <div class="showcase-columns" style="margin-top: 20px">
        <div style="flex: 1.4; min-width: 300px">
          <LumCodeBlock><span class="com">// theme tokens are just CSS variables</span>
<span class="kw">fn</span> <span class="fn">main</span>() {
  <span class="kw">let</span> <span class="var">version</span> = <span class="const">APP_VERSION</span>;
  <span class="fn">println!</span>(<span class="str">"Luma {}"</span>, <span class="var">version</span>);
}</LumCodeBlock>
          <p class="showcase-row" style="margin-top: 12px">
            Inline <code class="lum-code">code</code> and <LumKbd keys="Ctrl+Shift+P" />
          </p>
        </div>

        <div style="flex: 1; min-width: 280px">
          <LumCodeBlock><span style="background: var(--lum-diff-add-bg); display: block">+ added line</span><span style="background: var(--lum-diff-remove-bg); display: block">- removed line</span><span style="background: var(--lum-diff-modify-bg); display: block">~ modified line</span></LumCodeBlock>
        </div>
      </div>

      <div class="showcase-row" style="margin-top: 16px">
        <LumButton variant="primary" @click="toast('success', 'Saved', 'Your changes are safe.')">Notify success</LumButton>
        <LumButton variant="primary" @click="toast('info', 'FYI', 'Something informative.')">Notify info</LumButton>
        <LumButton variant="primary" @click="toast('warning', 'Careful', 'Potential danger ahead.')">Notify warning</LumButton>
        <LumButton variant="primary" @click="toast('danger', 'Failed', 'Something broke.')">Notify danger</LumButton>
      </div>
    </section>

    <footer class="lum-caption" style="padding: 24px 0 48px; text-align: center">
      Themeable everything — override <code class="lum-code">--lum-*</code> variables to restyle any component.
    </footer>

    <LumToasts />
  </div>
</template>

<style scoped>
/* showcase chrome only — components bring their own styles */
.showcase {
  height: 100vh;
  max-width: 1100px;
  margin: 0 auto;
  padding: 24px 32px 24px;
  overflow-y: auto;
}

.showcase-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 16px;
  padding-bottom: 16px;
  margin-bottom: 8px;
  border-bottom: 1px solid var(--lum-border);
}

.showcase-section {
  margin-top: 40px;
}

.showcase-section > h2 {
  display: flex;
  align-items: center;
  gap: 10px;
  margin-bottom: 14px;
  font-size: var(--lum-text-lg);
  color: var(--lum-fg-muted);
  text-transform: uppercase;
  letter-spacing: 0.5px;
}

.showcase-section > h2::before {
  content: "";
  width: 3px;
  height: 16px;
  border-radius: 2px;
  background: var(--lum-accent);
}

.showcase-row {
  display: flex;
  flex-wrap: wrap;
  align-items: center;
  gap: 12px;
  margin-bottom: 12px;
}

.showcase-columns {
  display: flex;
  gap: 32px;
  flex-wrap: wrap;
}

.relative-demo {
  position: relative;
}

.absolute-demo {
  position: absolute;
  top: calc(100% + 4px);
  left: 0;
}
</style>
