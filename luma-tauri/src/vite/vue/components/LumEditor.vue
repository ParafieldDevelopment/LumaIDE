<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from "vue";
import * as monaco from "monaco-editor";
import "../../node_modules/monaco-editor/min/vs/editor/editor.main.css";

monaco.editor.setTheme("vs-dark");

const props = withDefaults(
  defineProps<{ language?: string; readOnly?: boolean; modelKey?: string | number }>(),
  { readOnly: false, modelKey: "default" }
);
const value = defineModel<string>({ default: "" });
const emit = defineEmits<{ load: [editor: monaco.editor.IStandaloneCodeEditor] }>();

const editorRef = ref<HTMLDivElement | null>(null);
let editor: monaco.editor.IStandaloneCodeEditor | null = null;
let editorModel: monaco.editor.ITextModel | null = null;
let isUpdating = false;
let resizeObserver: ResizeObserver | null = null;
let contentListener: monaco.IDisposable | null = null;

const models = new Map<string, monaco.editor.ITextModel>();
const viewStates = new Map<string, monaco.editor.ICodeEditorViewState | null>();

function modelKey(): string {
  return String(props.modelKey);
}

function modelUri(key: string): monaco.Uri {
  return monaco.Uri.parse(`inmemory://luma-editor/${encodeURIComponent(key)}`);
}

function getOrCreateModel(key: string): monaco.editor.ITextModel {
  const existing = models.get(key);
  if (existing && !existing.isDisposed()) return existing;
  const created = monaco.editor.createModel(value.value, props.language || "plaintext", modelUri(key));
  models.set(key, created);
  return created;
}

function keyForModel(model: monaco.editor.ITextModel): string {
  for (const [key, stored] of models) if (stored === model) return key;
  return modelKey();
}

function switchModel() {
  if (!editor) return;
  const key = modelKey();
  const nextModel = getOrCreateModel(key);
  if (editorModel === nextModel) {
    if (props.language && nextModel.getLanguageId() !== props.language)
      monaco.editor.setModelLanguage(nextModel, props.language);
    return;
  }
  if (editorModel) viewStates.set(keyForModel(editorModel), editor.saveViewState());
  editor.setModel(nextModel);
  editorModel = nextModel;
  if (props.language && nextModel.getLanguageId() !== props.language)
    monaco.editor.setModelLanguage(nextModel, props.language);
  const state = viewStates.get(key);
  if (state) editor.restoreViewState(state);
  else editor.setPosition({ lineNumber: 1, column: 1 });
  layoutEditor();
}

function layoutEditor() {
  if (!editor || !editorRef.value) return;
  const rect = editorRef.value.getBoundingClientRect();
  if (rect.width > 0 && rect.height > 0) {
    editor.layout({ width: rect.width, height: rect.height });
  }
}

function createEditor() {
  if (!editorRef.value) return;
  editor = monaco.editor.create(editorRef.value, {
    readOnly: props.readOnly,
    fontFamily: '"JetBrains Mono", monospace',
    fontSize: 14,
    lineNumbers: "on",
    minimap: { enabled: true },
    automaticLayout: false,
    scrollBeyondLastLine: false,
    wordWrap: "on",
  });
  switchModel();
  requestAnimationFrame(() => layoutEditor());
  contentListener = editor.onDidChangeModelContent(() => {
    if (isUpdating || !editor) return;
    value.value = editor.getValue();
  });
  resizeObserver = new ResizeObserver(layoutEditor);
  resizeObserver.observe(editorRef.value);
  window.addEventListener("resize", layoutEditor);
  requestAnimationFrame(layoutEditor);
  emit("load", editor);
}

onMounted(createEditor);
watch(() => props.modelKey, switchModel);

watch(() => value.value, (next) => {
  if (!editorModel || editorModel.isDisposed() || editorModel.getValue() === next) return;
  isUpdating = true;
  try { editorModel.setValue(next); }
  finally { isUpdating = false; }
});

watch(() => props.language, (next) => {
  if (editorModel && next && editorModel.getLanguageId() !== next)
    monaco.editor.setModelLanguage(editorModel, next);
});
watch(() => props.readOnly, (next) => editor?.updateOptions({ readOnly: next }));

onBeforeUnmount(() => {
  if (editor && editorModel) viewStates.set(keyForModel(editorModel), editor.saveViewState());
  resizeObserver?.disconnect();
  resizeObserver = null;
  window.removeEventListener("resize", layoutEditor);
  contentListener?.dispose();
  contentListener = null;
  editor?.dispose();
  editor = null;
  editorModel = null;
  for (const model of models.values()) if (!model.isDisposed()) model.dispose();
  models.clear();
  viewStates.clear();
});
</script>

<template><div ref="editorRef" class="lum-editor" /></template>

<style scoped>
.lum-editor { width: 100%; height: 100%; min-height: 0; display: block; overflow: hidden; }
</style>