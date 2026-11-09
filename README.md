> [!WARNING]
> # PRE-ALPHA WARNING!
> This project is still in development and is not yet ready for use.
> 
> Quite literally nothing is completed!

# Luma IDE

An Integrated Development Environment (IDE) for everything Lua/Luau related.

## Monaco Editor

The Monaco Editor fork is tracked as a Git submodule at `luma-tauri/src/vite/editor`.
Initialize it after cloning the repository:

```sh
git submodule update --init --recursive
```

With Node.js and npm installed, build the fork from the repository root:

```sh
npm --prefix luma-tauri/src/vite run build:editor
```

The build uses the fork's lockfile and writes the distributable to
`luma-tauri/src/vite/editor/out/monaco-editor`.