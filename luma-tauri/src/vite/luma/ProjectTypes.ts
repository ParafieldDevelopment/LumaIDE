/**
 * Project type registry for the New Workspace wizard.
 *
 * Data-driven on purpose: the wizard shell knows nothing about any specific
 * runtime. Adding Lua 5.5, Defold, Love2D, etc. means adding an entry here
 * (later: contributed by an extension), not editing the wizard UI.
 *
 * NOTE: runtime and template choices are not applied by the backend yet.
 * `create_workspace` currently only receives a title and parent path.
 */

import { ref } from "vue";

export interface ProjectRuntime {
    id: string;
    label: string;
}

export interface ProjectTemplate {
    id: string;
    label: string;
    description: string;
}

/** Built-in groups are listed below; extensions may introduce any group name. */
export type ProjectTypeGroup = string;

export interface ProjectType {
    id: string;
    label: string;
    group: ProjectTypeGroup;
    /** Short text for the list badge (about 3 characters). Defaults to the label's initials. */
    badge?: string;
    /** Badge background (any CSS color). Defaults to a color derived from the label. */
    color?: string;
    /** One or two sentences shown at the top of the right-hand pane. */
    description: string;
    runtimes: ProjectRuntime[];
    templates: ProjectTemplate[];
}

const GENERIC_TEMPLATES: ProjectTemplate[] = [
    { id: "empty", label: "Empty", description: "Workspace with no source files." },
    { id: "hello-world", label: "Hello World", description: "A single main script that prints a message." },
    { id: "library", label: "Library", description: "A reusable module with an entry point and a usage example." },
];

const BUILTIN_PROJECT_TYPES: ProjectType[] = [
    {
        id: "lua",
        label: "Lua",
        group: "Lua runtimes",
        badge: "Lua",
        color: "#3f5fd0",
        description: "Standard Lua scripts and libraries, run with a Lua interpreter.",
        runtimes: [
            { id: "lua-5.4", label: "Lua 5.4" },
            { id: "lua-5.3", label: "Lua 5.3" },
            { id: "lua-5.2", label: "Lua 5.2" },
            { id: "lua-5.1", label: "Lua 5.1" },
        ],
        templates: GENERIC_TEMPLATES,
    },
    {
        id: "luajit",
        label: "LuaJIT",
        group: "Lua runtimes",
        badge: "JIT",
        color: "#1d9e86",
        description: "Lua projects targeting LuaJIT, including FFI-based code.",
        runtimes: [
            { id: "luajit-2.1", label: "LuaJIT 2.1" },
            { id: "luajit-2.0", label: "LuaJIT 2.0" },
        ],
        templates: GENERIC_TEMPLATES,
    },
    {
        id: "luau",
        label: "Luau",
        group: "Lua runtimes",
        badge: "Luau",
        color: "#7a5ad0",
        description: "Standalone Luau projects with type annotations, outside Roblox.",
        runtimes: [{ id: "luau-latest", label: "Luau (latest)" }],
        templates: GENERIC_TEMPLATES,
    },
    {
        id: "roblox",
        label: "Roblox",
        group: "Platforms",
        description: "Luau projects for Roblox experiences, libraries and Studio plugins.",
        runtimes: [{ id: "roblox-studio", label: "Roblox Studio (Luau)" }],
        templates: [
            { id: "empty", label: "Empty", description: "Workspace with no source files." },
            { id: "module-library", label: "Module library", description: "A ModuleScript-style library with a test script." },
            { id: "plugin", label: "Studio plugin", description: "A plugin entry script for Roblox Studio." },
        ],
    },
    {
        id: "computercraft",
        label: "ComputerCraft",
        group: "Platforms",
        description: "Programs for computers and turtles in CC:Tweaked.",
        runtimes: [{ id: "cc-tweaked", label: "CC:Tweaked" }],
        templates: [
            { id: "empty", label: "Empty", description: "Workspace with no source files." },
            { id: "program", label: "Computer program", description: "A program with a startup script." },
            { id: "turtle", label: "Turtle program", description: "A turtle program skeleton." },
            { id: "api", label: "API library", description: "A reusable library loaded with require." },
        ],
    },
    {
        id: "empty",
        label: "Empty Workspace",
        group: "Other",
        badge: "Em",
        color: "#6b7280",
        description: "A plain workspace folder. Choose a runtime later.",
        runtimes: [],
        templates: [{ id: "empty", label: "Empty", description: "Workspace with no source files." }],
    },
];

/** Reactive, so the wizard updates if an extension registers a type while it is open. */
const registry = ref<ProjectType[]>([...BUILTIN_PROJECT_TYPES]);

/**
 * Register a project type (intended for extensions).
 * Re-registering an existing id replaces it. Returns an unregister function.
 */
export function registerProjectType(type: ProjectType): () => void {
    registry.value = [...registry.value.filter(existing => existing.id !== type.id), type];
    return () => {
        registry.value = registry.value.filter(existing => existing !== type);
    };
}

export function listProjectTypes(): ProjectType[] {
    return registry.value;
}

export interface ProjectTypeGroupEntry {
    label: ProjectTypeGroup;
    types: ProjectType[];
}

const FIRST_GROUPS: ProjectTypeGroup[] = ["Lua runtimes", "Platforms"];
const LAST_GROUPS: ProjectTypeGroup[] = ["Other"];

/** Built-in groups first, then extension groups in registration order, then "Other". */
export function groupedProjectTypes(): ProjectTypeGroupEntry[] {
    const all = registry.value;
    const extensionGroups = [...new Set(all.map(type => type.group))]
        .filter(group => !FIRST_GROUPS.includes(group) && !LAST_GROUPS.includes(group));
    return [...FIRST_GROUPS, ...extensionGroups, ...LAST_GROUPS]
        .map(label => ({ label, types: all.filter(type => type.group === label) }))
        .filter(group => group.types.length > 0);
}

export function getProjectType(id: string): ProjectType {
    return registry.value.find(type => type.id === id) ?? registry.value[0];
}
