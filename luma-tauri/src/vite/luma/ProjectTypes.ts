/**
 * Project type registry for the New Workspace wizard.
 *
 * Data-driven on purpose: the wizard shell knows nothing about any specific
 * runtime. Adding a new language version, engine or platform means adding an
 * entry here (later: contributed by an extension), not editing the wizard UI.
 *
 * The chosen type, runtime and template ids are saved in the workspace's
 * `.luma/project.json` and the recent list, and template files are generated
 * into the newly created workspace.
 */

import { ref } from "vue";

export interface ProjectRuntime {
    id: string;
    label: string;
    /** Icon path under Vite's `public/project_type_icons/` directory. */
    icon?: string;
}

export interface TemplateFile {
    path: string;
    content: string;
}

export interface TemplateContext {
    title: string;
    projectType: string;
    runtime?: string;
    template: string;
}

export interface ProjectTemplate {
    id: string;
    label: string;
    description: string;
    generate?: (context: TemplateContext) => TemplateFile[];
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
    /** Icon path under Vite's `public/project_type_icons/` directory. */
    icon?: string;
    /** One or two sentences shown at the top of the right-hand pane. */
    description: string;
    runtimes: ProjectRuntime[];
    templates: ProjectTemplate[];
}

function toIdentifier(name: string): string {
    const cleaned = name.replace(/[^a-zA-Z0-9_]/g, "");
    if (!cleaned || /^[0-9]/.test(cleaned)) {
        return "App";
    }
    return cleaned;
}

/* --------------------------------- Runtimes ---------------------------------- */

const LUA_ICON = "/project_type_icons/lua.svg";
const LUAU_ICON = "/project_type_icons/luau.svg";
const CC_ICON = "/project_type_icons/cc.png";

const LUA_VERSION_RUNTIMES: ProjectRuntime[] = [
    { id: "lua-5.5", label: "Lua (PUC) 5.5", icon: LUA_ICON },
    { id: "lua-5.4", label: "Lua (PUC) 5.4", icon: LUA_ICON },
    { id: "lua-5.3", label: "Lua (PUC) 5.3", icon: LUA_ICON },
    { id: "lua-5.2", label: "Lua (PUC) 5.2", icon: LUA_ICON },
    { id: "lua-5.1", label: "Lua (PUC) 5.1", icon: LUA_ICON },
];

const LUA_ENGINE_RUNTIMES: ProjectRuntime[] = [
    { id: "love2d", label: "LÖVE", icon: "/project_type_icons/love.svg" },
    { id: "defold", label: "Defold", icon: "/project_type_icons/defold.svg"  },
    { id: "openresty", label: "OpenResty", icon: "/project_type_icons/openresty.webp"  },
    { id: "tarantool", label: "Tarantool", icon: "/project_type_icons/tarantool.svg" },
];

const LUA_RUNTIMES: ProjectRuntime[] = [...LUA_VERSION_RUNTIMES, ...LUA_ENGINE_RUNTIMES];

const LUAJIT_RUNTIMES: ProjectRuntime[] = [
    { id: "luajit-2.1", label: "LuaJIT 2.1", icon: LUA_ICON },
    { id: "luajit-2.0", label: "LuaJIT 2.0", icon: LUA_ICON },
    { id: "openresty-luajit", label: "OpenResty (LuaJIT)", icon: "/project_type_icons/openresty.webp" },
    { id: "love2d-luajit", label: "LÖVE (LuaJIT)", icon: "/project_type_icons/love.svg" },
];

const LUAU_RUNTIMES: ProjectRuntime[] = [
    { id: "lune", label: "Lune", icon: LUAU_ICON },
    { id: "lute", label: "Lute", icon: "/project_type_icons/lune.svg" },
    { id: "seal", label: "Seal", icon: "/project_type_icons/seal.png" },
    { id: "zune", label: "Zune", icon: LUAU_ICON },
];

/* --------------------------------- Templates --------------------------------- */

const GENERIC_TEMPLATES: ProjectTemplate[] = [
    { id: "empty", label: "Empty", description: "Workspace with no source files." },
    {
        id: "hello-world",
        label: "Hello World",
        description: "A single main script that prints a message.",
        generate(context: TemplateContext): TemplateFile[] {
            if (context.projectType === "luau") {
                return [
                    {
                        path: "main.luau",
                        content: [
                            "--!strict",
                            `-- main.luau (${context.title})`,
                            "",
                            "local function greet(name: string): string",
                            "    return `Hello, {name}!`",
                            "end",
                            "",
                            'print(greet("Luau"))',
                            "",
                        ].join("\n"),
                    },
                ];
            }

            if (context.projectType === "luajit") {
                return [
                    {
                        path: "main.lua",
                        content: [
                            `-- main.lua (${context.title})`,
                            "-- LuaJIT runtime",
                            "",
                            'local ffi = require("ffi")',
                            "",
                            'print(string.format("Running %s on %s", jit.version, jit.os))',
                            'print("Hello from LuaJIT!")',
                            "",
                        ].join("\n"),
                    },
                ];
            }

            return [
                {
                    path: "main.lua",
                    content: [
                        `-- main.lua (${context.title})`,
                        "",
                        "local function main()",
                        '    print("Hello, World!")',
                        "end",
                        "",
                        "main()",
                        "",
                    ].join("\n"),
                },
            ];
        },
    },
    {
        id: "library",
        label: "Library",
        description: "A reusable module with an entry point and a usage example.",
        generate(context: TemplateContext): TemplateFile[] {
            if (context.projectType === "luau") {
                return [
                    {
                        path: "src/lib.luau",
                        content: [
                            "--!strict",
                            `-- src/lib.luau (${context.title})`,
                            "",
                            "export type Greeter = {",
                            "    prefix: string,",
                            "}",
                            "",
                            "local Lib = {}",
                            "",
                            "function Lib.new(prefix: string?): Greeter",
                            "    return {",
                            '        prefix = prefix or "Hello",',
                            "    }",
                            "end",
                            "",
                            "function Lib.greet(self: Greeter, target: string?): string",
                            '    local who = target or "World"',
                            "    return `{self.prefix}, {who}!`",
                            "end",
                            "",
                            "return Lib",
                            "",
                        ].join("\n"),
                    },
                    {
                        path: "main.luau",
                        content: [
                            "--!strict",
                            `-- main.luau (${context.title})`,
                            "",
                            'local Lib = require("./src/lib")',
                            "",
                            'local greeter = Lib.new("Welcome")',
                            'print(greeter:greet("Luau"))',
                            "",
                        ].join("\n"),
                    },
                ];
            }

            return [
                {
                    path: "src/lib.lua",
                    content: [
                        `-- src/lib.lua (${context.title})`,
                        "",
                        "local M = {}",
                        "",
                        "function M.greet(name)",
                        '    local target = name or "World"',
                        '    return string.format("Hello, %s!", target)',
                        "end",
                        "",
                        "function M.add(a, b)",
                        "    return a + b",
                        "end",
                        "",
                        "return M",
                        "",
                    ].join("\n"),
                },
                {
                    path: "main.lua",
                    content: [
                        `-- main.lua (${context.title})`,
                        "",
                        'local lib = require("src.lib")',
                        "",
                        'print(lib.greet("Developer"))',
                        'print("2 + 3 =", lib.add(2, 3))',
                        "",
                    ].join("\n"),
                },
            ];
        },
    },
];

const ROBLOX_TEMPLATES: ProjectTemplate[] = [
    { id: "empty", label: "Empty", description: "Workspace with no source files." },
    {
        id: "module-library",
        label: "Module library",
        description: "A ModuleScript-style library with a test script.",
        generate(context: TemplateContext): TemplateFile[] {
            const ident = toIdentifier(context.title);
            return [
                {
                    path: "default.project.json",
                    content:
                        JSON.stringify(
                            {
                                name: context.title,
                                tree: {
                                    $className: "DataModel",
                                    ReplicatedStorage: {
                                        [context.title]: {
                                            $path: "src",
                                        },
                                    },
                                },
                            },
                            null,
                            2,
                        ) + "\n",
                },
                {
                    path: "src/init.luau",
                    content: [
                        "--!strict",
                        `-- ModuleScript: ${context.title}`,
                        "",
                        `local ${ident} = {}`,
                        "",
                        "function " + ident + ".greet(name: string?): string",
                        '    local target = name or "Roblox"',
                        "    return `Hello, {target}!`",
                        "end",
                        "",
                        `return ${ident}`,
                        "",
                    ].join("\n"),
                },
                {
                    path: "tests/test.server.luau",
                    content: [
                        "--!strict",
                        `-- Test script for ${context.title}`,
                        "",
                        'local ReplicatedStorage = game:GetService("ReplicatedStorage")',
                        `local ${ident} = require(ReplicatedStorage:WaitForChild("${context.title}"))`,
                        "",
                        `print(${ident}.greet())`,
                        "",
                    ].join("\n"),
                },
            ];
        },
    },
    {
        id: "plugin",
        label: "Studio plugin",
        description: "A plugin entry script for Roblox Studio.",
        generate(context: TemplateContext): TemplateFile[] {
            return [
                {
                    path: "default.project.json",
                    content:
                        JSON.stringify(
                            {
                                name: context.title,
                                tree: {
                                    $className: "DataModel",
                                    Plugin: {
                                        $path: "src",
                                    },
                                },
                            },
                            null,
                            2,
                        ) + "\n",
                },
                {
                    path: "src/init.server.luau",
                    content: [
                        "--!strict",
                        `-- Roblox Studio Plugin: ${context.title}`,
                        "",
                        `local toolbar = plugin:CreateToolbar("${context.title}")`,
                        `local button = toolbar:CreateButton(`,
                        `    "Toggle ${context.title}",`,
                        `    "Open or close ${context.title}",`,
                        '    "rbxassetid://4458901886"',
                        ")",
                        "",
                        "button.Click:Connect(function()",
                        `    print("${context.title} button clicked!")`,
                        "end)",
                        "",
                    ].join("\n"),
                },
            ];
        },
    },
];

/* ------------------------------- Project types ------------------------------- */

const BUILTIN_PROJECT_TYPES: ProjectType[] = [
    {
        id: "lua",
        label: "Lua",
        group: "Lua runtimes",
        badge: "Lua",
        icon: LUA_ICON,
        description: "Standard Lua scripts and libraries, run with a Lua interpreter.",
        runtimes: LUA_RUNTIMES,
        templates: GENERIC_TEMPLATES,
    },
    {
        id: "luajit",
        label: "LuaJIT",
        group: "Lua runtimes",
        badge: "JIT",
        icon: LUA_ICON,
        description: "Lua projects targeting LuaJIT, including FFI-based code.",
        runtimes: LUAJIT_RUNTIMES,
        templates: GENERIC_TEMPLATES,
    },
    {
        id: "luau",
        label: "Luau",
        group: "Lua runtimes",
        badge: "Luau",
        icon: LUAU_ICON,
        description: "Standalone Luau projects with type annotations, outside Roblox.",
        runtimes: LUAU_RUNTIMES,
        templates: GENERIC_TEMPLATES,
    },
    {
        id: "roblox",
        label: "Roblox",
        group: "Platforms",
        icon: "/project_type_icons/robloxstudio.svg",
        description: "Luau projects for Roblox experiences, libraries and Studio plugins.",
        runtimes: [
            { id: "luma_sync", label: "Luma Sync", icon: "/project_type_icons/lumasync.svg" },
            { id: "rojo", label: "Rojo (Rokit)", icon: "/project_type_icons/rojo.png" },
        ],
        templates: ROBLOX_TEMPLATES,
    },
    {
        id: "computercraft",
        label: "ComputerCraft",
        group: "Platforms",
        icon: CC_ICON,
        description: "Programs for computers and turtles in CC:Tweaked.",
        runtimes: [
            { id: "cc-tweaked", label: "CC: Tweaked", icon: CC_ICON }
        ],
        templates: [
            { id: "empty", label: "Empty", description: "Workspace with no source files." },
            {
                id: "program",
                label: "Computer program",
                description: "A program with a startup script.",
                generate(context: TemplateContext): TemplateFile[] {
                    return [
                        {
                            path: "startup.lua",
                            content: [
                                `-- startup.lua (${context.title})`,
                                "-- CC:Tweaked Computer program",
                                "",
                                "term.clear()",
                                "term.setCursorPos(1, 1)",
                                "",
                                `print("=== ${context.title} ===")`,
                                'print("Press any key to continue...")',
                                'os.pullEvent("key")',
                                "",
                                'print("Program finished.")',
                                "",
                            ].join("\n"),
                        },
                    ];
                },
            },
            {
                id: "turtle",
                label: "Turtle program",
                description: "A turtle program skeleton.",
                generate(context: TemplateContext): TemplateFile[] {
                    return [
                        {
                            path: "startup.lua",
                            content: [
                                `-- startup.lua (${context.title})`,
                                "-- CC:Tweaked Turtle program",
                                "",
                                "if not turtle then",
                                '    printError("Error: This program must be run on a Turtle!")',
                                "    return",
                                "end",
                                "",
                                "term.clear()",
                                "term.setCursorPos(1, 1)",
                                "",
                                `print("=== Turtle: ${context.title} ===")`,
                                'print("Fuel level: " .. tostring(turtle.getFuelLevel()) .. "/" .. tostring(turtle.getFuelLimit()))',
                                "",
                                "if turtle.getFuelLevel() == 0 then",
                                '    print("Warning: Turtle needs fuel to move.")',
                                "end",
                                "",
                            ].join("\n"),
                        },
                    ];
                },
            },
            {
                id: "api",
                label: "API library",
                description: "A reusable library loaded with require.",
                generate(context: TemplateContext): TemplateFile[] {
                    const ident = toIdentifier(context.title);
                    return [
                        {
                            path: `src/${ident}.lua`,
                            content: [
                                `-- src/${ident}.lua (${context.title})`,
                                "-- CC:Tweaked API module",
                                "",
                                "local api = {}",
                                "",
                                "function api.greet(name)",
                                '    return "Hello, " .. (name or "CC User") .. "!"',
                                "end",
                                "",
                                "return api",
                                "",
                            ].join("\n"),
                        },
                        {
                            path: "startup.lua",
                            content: [
                                `-- startup.lua (${context.title})`,
                                "",
                                `local api = require("src/${ident}")`,
                                "",
                                "term.clear()",
                                "term.setCursorPos(1, 1)",
                                `print("=== ${context.title} ===")`,
                                "print(api.greet())",
                                "",
                            ].join("\n"),
                        },
                    ];
                },
            },
        ],
    },
    {
        id: "empty",
        label: "Empty Workspace",
        group: "Other",
        badge: "Em",
        color: "#6b7280",
        icon: "/project_type_icons/box.svg",
        description: "A plain workspace folder. Choose a runtime later.",
        runtimes: [],
        templates: [{ id: "empty", label: "Empty", description: "Workspace with no source files." }],
    },
];

/* --------------------------------- Registry ---------------------------------- */

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

export function getTemplateFiles(
    typeId: string,
    templateId: string,
    context: { title: string; runtime?: string },
): TemplateFile[] {
    const type = getProjectType(typeId);
    const template = type.templates.find(t => t.id === templateId);
    if (!template?.generate) {
        return [];
    }
    return template.generate({
        title: context.title,
        projectType: typeId,
        runtime: context.runtime,
        template: templateId,
    });
}
