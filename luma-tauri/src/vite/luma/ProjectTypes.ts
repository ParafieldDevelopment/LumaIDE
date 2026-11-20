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

export interface ProjectRuntime {
    id: string;
    label: string;
}

export interface ProjectTemplate {
    id: string;
    label: string;
    description: string;
}

export type ProjectTypeGroup = "Lua runtimes" | "Platforms" | "Other";

export interface ProjectType {
    id: string;
    label: string;
    group: ProjectTypeGroup;
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

export const PROJECT_TYPES: ProjectType[] = [
    {
        id: "lua",
        label: "Lua",
        group: "Lua runtimes",
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
        description: "A plain workspace folder. Choose a runtime later.",
        runtimes: [],
        templates: [{ id: "empty", label: "Empty", description: "Workspace with no source files." }],
    },
];

export interface ProjectTypeGroupEntry {
    label: ProjectTypeGroup;
    types: ProjectType[];
}

const GROUP_ORDER: ProjectTypeGroup[] = ["Lua runtimes", "Platforms", "Other"];

export function groupedProjectTypes(): ProjectTypeGroupEntry[] {
    return GROUP_ORDER
        .map(label => ({ label, types: PROJECT_TYPES.filter(type => type.group === label) }))
        .filter(group => group.types.length > 0);
}

export function getProjectType(id: string): ProjectType {
    return PROJECT_TYPES.find(type => type.id === id) ?? PROJECT_TYPES[0];
}
