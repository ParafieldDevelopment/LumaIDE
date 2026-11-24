<script setup lang="ts">
import { ref, onMounted, onBeforeUnmount, watch } from "vue";
import * as monaco from "monaco-editor";
import "../../node_modules/monaco-editor/min/vs/editor/editor.main.css";
import { getActionContext, registerActions, updateActionContext } from "~/vue/composables/actions";
import { registerContextMenu } from "~/vue/composables/contextMenu";
import { getMonacoContextMenuItems } from "~/vue/composables/monacoContextMenu";

monaco.editor.setTheme("vs-dark");

interface EditorCommand {
  id: string;
  label: string;
  shortcut?: string;
  icon?: string;
  group: string;
  order: number;
  isChecked?: () => boolean;
  run: (editor: monaco.editor.IStandaloneCodeEditor) => void;
}

let minimapEnabled = true;
let wordWrapEnabled = true;

const editorCommands: EditorCommand[] = [
  { id: "editor.undo", label: "Undo", shortcut: "Ctrl+Z", group: "01_history", order: 10,
    run: (editor) => editor.trigger("menu", "undo", null) },
  { id: "editor.redo", label: "Redo", shortcut: "Ctrl+Y", group: "01_history", order: 20,
    run: (editor) => editor.trigger("menu", "redo", null) },
  { id: "editor.cut", label: "Cut", shortcut: "Ctrl+X", group: "02_clipboard", order: 10,
    run: (editor) => editor.trigger("menu", "cut", null) },
  { id: "editor.copy", label: "Copy", shortcut: "Ctrl+C", group: "02_clipboard", order: 20,
    run: (editor) => editor.trigger("menu", "copy", null) },
  { id: "editor.paste", label: "Paste", shortcut: "Ctrl+V", group: "02_clipboard", order: 30,
    run: (editor) => editor.trigger("menu", "paste", null) },
  { id: "editor.selectAll", label: "Select All", shortcut: "Ctrl+A", group: "02_clipboard", order: 40,
    run: (editor) => editor.trigger("menu", "editor.action.selectAll", null) },
  { id: "editor.formatDocument", label: "Format Document", shortcut: "Shift+Alt+F", group: "03_format", order: 10,
    run: (editor) => { void editor.getAction("editor.action.formatDocument")?.run(); } },
  { id: "editor.commentLine", label: "Toggle Line Comment", shortcut: "Ctrl+/", group: "03_format", order: 20,
    run: (editor) => { void editor.getAction("editor.action.commentLine")?.run(); } },
  { id: "editor.toggleWordWrap", label: "Word Wrap", group: "04_view", order: 10,
    isChecked: () => wordWrapEnabled,
    run: (editor) => {
      wordWrapEnabled = !wordWrapEnabled;
      editor.updateOptions({ wordWrap: wordWrapEnabled ? "on" : "off" });
    } },
  { id: "editor.toggleMinimap", label: "Minimap", group: "04_view", order: 20,
    isChecked: () => minimapEnabled,
    run: (editor) => {
      minimapEnabled = !minimapEnabled;
      editor.updateOptions({ minimap: { enabled: minimapEnabled } });
      requestAnimationFrame(layoutEditor);
    } },
];

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
let disposeActions: (() => void) | null = null;
let disposeContextMenu: (() => void) | null = null;

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

function onFontsLoaded() {
  monaco.editor.remeasureFonts();
  layoutEditor();
}

function onWindowFocus() {
  monaco.editor.remeasureFonts();
  layoutEditor();
}

async function createEditor() {
  if (!editorRef.value) return;

  // Await web font loading before Monaco measures font metrics
  if ("fonts" in document) {
    try {
      await document.fonts.load('14px "JetBrains Mono"');
      await document.fonts.ready;
    } catch {
      // Graceful fallback if font loading fails
    }
  }

  editor = monaco.editor.create(editorRef.value, {
    readOnly: props.readOnly,
    fontFamily: '"JetBrains Mono", monospace',
    fontSize: 14,
    lineNumbers: "on",
    minimap: { enabled: minimapEnabled },
    automaticLayout: false,
    scrollBeyondLastLine: false,
    wordWrap: wordWrapEnabled ? "on" : "off",
    letterSpacing: 0,
    fontLigatures: false,
    // The IDE renders its own unified context menu instead.
    contextmenu: false,
  });

  monaco.editor.remeasureFonts();
  switchModel();
  requestAnimationFrame(() => layoutEditor());

  contentListener = editor.onDidChangeModelContent(() => {
    if (isUpdating || !editor) return;
    value.value = editor.getValue();
  });

  resizeObserver = new ResizeObserver(layoutEditor);
  resizeObserver.observe(editorRef.value);
  window.addEventListener("resize", layoutEditor);
  window.addEventListener("focus", onWindowFocus);

  if ("fonts" in document) {
    document.fonts.addEventListener("loadingdone", onFontsLoaded);
  }

  requestAnimationFrame(layoutEditor);

  // Editor commands become available everywhere: context menu, palette, menus.
  disposeActions = registerActions(editorCommands.map((command) => ({
    id: command.id,
    label: command.label,
    shortcut: command.shortcut,
    icon: command.icon,
    category: "Editor",
    when: (context) => !!context.editor,
    checked: command.isChecked ? () => !!command.isChecked?.() : undefined,
    run: (context) => { if (context.editor) command.run(context.editor); },
  })));
  updateActionContext({ editor });
  emit("load", editor);
}

onMounted(() => {
  if (editorRef.value) {
    disposeContextMenu = registerContextMenu(editorRef.value, (event) => getMonacoContextMenuItems(editor, event));
  }
  void createEditor();
});
watch(() => props.modelKey, switchModel);

watch(() => value.value, (next) => {
  if (!editorModel || editorModel.isDisposed() || editorModel.getValue() === next) return;
  isUpdating = true;
  try {
    const state = editor?.saveViewState();
    editorModel.setValue(next);
    if (state && editor) editor.restoreViewState(state);
  }
  finally { isUpdating = false; }
});

watch(() => props.language, (next) => {
  if (editorModel && next && editorModel.getLanguageId() !== next)
    monaco.editor.setModelLanguage(editorModel, next);
});
watch(() => props.readOnly, (next) => editor?.updateOptions({ readOnly: next }));

onBeforeUnmount(() => {
  if (editor && editorModel) viewStates.set(keyForModel(editorModel), editor.saveViewState());
  if (editor && getActionContext().editor === editor) updateActionContext({ editor: null });
  disposeActions?.();
  disposeContextMenu?.();
  resizeObserver?.disconnect();
  resizeObserver = null;
  window.removeEventListener("resize", layoutEditor);
  window.removeEventListener("focus", onWindowFocus);
  if ("fonts" in document) {
    document.fonts.removeEventListener("loadingdone", onFontsLoaded);
  }
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
.lum-editor {
  width: 100%;
  height: 100%;
  min-height: 0;
  display: block;
  overflow: hidden;
}

.lum-editor :deep(.monaco-editor) {
  text-rendering: auto !important;
  letter-spacing: normal !important;
}

.lum-editor :deep(.monaco-editor .view-line),
.lum-editor :deep(.monaco-editor .view-lines),
.lum-editor :deep(.monaco-editor .lines-content) {
  letter-spacing: normal !important;
  font-variant-ligatures: none !important;
  text-rendering: auto !important;
}
</style>
