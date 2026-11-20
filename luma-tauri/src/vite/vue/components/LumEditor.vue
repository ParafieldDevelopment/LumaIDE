<script setup lang="ts">
/**
 * LumEditor — Monaco Editor wrapper (.lum-editor).
 * Two-way bound via v-model (modelValue / update:modelValue). Emits `load` when ready.
 */
import { ref, onMounted, onBeforeUnmount, watch } from "vue";
import * as monaco from "monaco-editor";
import "../../node_modules/monaco-editor/min/vs/editor/editor.main.css";

const props = withDefaults(
  defineProps<{
    language?: string;
    readOnly?: boolean;
  }>(),
  { readOnly: false }
);

const value = defineModel<string>({ default: "" });

const emit = defineEmits<{
  load: [editor: monaco.editor.IStandaloneCodeEditor];
}>();

const editorRef = ref<HTMLDivElement | null>(null);
let editor: monaco.editor.IStandaloneCodeEditor | null = null;
let editorModel: monaco.editor.ITextModel | null = null;
let isUpdating = false;

// Self-contained Monaco web worker environment. No external worker files are
// required, so nothing needs to be copied to the output directory.
if (typeof self !== "undefined" && !("MonacoEnvironment" in self)) {
  (self as Window & { MonacoEnvironment?: unknown }).MonacoEnvironment = {
    getWorkerUrl: () => {
      const blob = new Blob(
        ['self.postMessage({ type: "vscode-worker-ready" });'],
        { type: "application/javascript" }
      );
      return URL.createObjectURL(blob);
    },
  };
  monaco.editor.setTheme("vs-dark");
}

function createEditor() {
  if (!editorRef.value) return;

  const options: monaco.editor.IEditorConstructionOptions = {
    readOnly: props.readOnly,
    fontFamily: '"JetBrains Mono", monospace',
    fontSize: 14,
    lineNumbers: "on",
    minimap: { enabled: true },
    automaticLayout: true,
    scrollBeyondLastLine: false,
    wordWrap: "on",
  };

  editor = monaco.editor.create(editorRef.value, options);
  editorModel = editor.getModel();
  if (editorModel) {
    editorModel.setValue(value.value);
    if (props.language) {
      monaco.editor.setModelLanguage(editorModel, props.language);
    }
  }

  editor.onDidChangeModelContent(() => {
    if (isUpdating || !editor) return;
    value.value = editor.getValue();
  });

  emit("load", editor);
}

onMounted(() => {
  createEditor();
});

watch(
  () => value.value,
  (next) => {
    if (isUpdating || !editorModel) return;
    if (editorModel.getValue() !== next) {
      isUpdating = true;
      editorModel.setValue(next);
      isUpdating = false;
    }
  }
);

watch(
  () => props.language,
  (next) => {
    if (!editorModel || !next) return;
    monaco.editor.setModelLanguage(editorModel, next);
  }
);

onBeforeUnmount(() => {
  if (editor) {
    editor.dispose();
    editor = null;
  }
  if (editorModel) {
    editorModel.dispose();
    editorModel = null;
  }
});
</script>

<template>
  <div ref="editorRef" class="lum-editor" />
</template>

<style scoped>
.lum-editor {
  width: 100%;
  height: 100%;
  min-height: 0;
  display: block;
}
</style>
