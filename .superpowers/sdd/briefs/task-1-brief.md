# Task 1: Scaffold Tauri + React project

**Files:**
- Create: `package.json`, `vite.config.ts`, `tsconfig.json`, `tailwind.config.ts`, `postcss.config.js`, `src/styles/globals.css`, `src/main.tsx`, `index.html`
- Create: `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`, `src-tauri/build.rs`, `src-tauri/src/main.rs`, `src-tauri/src/lib.rs`, `src-tauri/icons/` (placeholder icons)
- Create: `src-tauri/capabilities/default.json`

**Interfaces:**
- Produces: a runnable `npm run tauri dev` that shows an empty window with "OpenCodeDeck" title.

## Steps

### Step 1: Initialize npm project and install frontend deps

Run from repo root:

```bash
npm init -y
npm install react react-dom
npm install -D typescript @types/react @types/react-dom vite @vitejs/plugin-react tailwindcss postcss autoprefixer @tauri-apps/cli @tauri-apps/api
```

### Step 2: Write package.json scripts

Overwrite `package.json` scripts section (preserve installed deps):

```json
{
  "name": "opencodedeck",
  "private": true,
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc && vite build",
    "preview": "vite preview",
    "tauri": "tauri"
  }
}
```

### Step 3: Write frontend config files

`tsconfig.json`:
```json
{
  "compilerOptions": {
    "target": "ES2021",
    "useDefineForClassFields": true,
    "lib": ["ES2021", "DOM", "DOM.Iterable"],
    "module": "ESNext",
    "skipLibCheck": true,
    "moduleResolution": "bundler",
    "allowImportingTsExtensions": true,
    "resolveJsonModule": true,
    "isolatedModules": true,
    "noEmit": true,
    "jsx": "react-jsx",
    "strict": true,
    "noUnusedLocals": true,
    "noUnusedParameters": true,
    "noFallthroughCasesInSwitch": true
  },
  "include": ["src"]
}
```

`vite.config.ts`:
```ts
import { defineConfig } from "vite"
import react from "@vitejs/plugin-react"

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  envPrefix: ["VITE_", "TAURI_"],
  build: { target: "es2021", minify: "esbuild", sourcemap: false },
})
```

`tailwind.config.ts`:
```ts
import type { Config } from "tailwindcss"

export default {
  content: ["./index.html", "./src/**/*.{ts,tsx}"],
  theme: { extend: {} },
  plugins: [],
} satisfies Config
```

`postcss.config.js`:
```js
export default {
  plugins: { tailwindcss: {}, autoprefixer: {} },
}
```

`index.html`:
```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="UTF-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1.0" />
    <title>OpenCodeDeck</title>
  </head>
  <body class="bg-background text-foreground">
    <div id="root"></div>
    <script type="module" src="/src/main.tsx"></script>
  </body>
</html>
```

`src/styles/globals.css`:
```css
@tailwind base;
@tailwind components;
@tailwind utilities;
```

`src/main.tsx`:
```tsx
import React from "react"
import ReactDOM from "react-dom/client"
import App from "./App"
import "./styles/globals.css"

ReactDOM.createRoot(document.getElementById("root")!).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
)
```

`src/App.tsx` (placeholder):
```tsx
export default function App() {
  return <div className="p-8 text-lg">OpenCodeDeck</div>
}
```

### Step 4: Scaffold Tauri Rust backend

`src-tauri/Cargo.toml`:
```toml
[package]
name = "opencodedeck"
version = "0.1.0"
edition = "2021"

[lib]
name = "opencodedeck_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
dirs = "5"
thiserror = "1"
reqwest = { version = "0.12", features = ["blocking"] }
chrono = "0.4"
```

`src-tauri/build.rs`:
```rust
fn main() {
    tauri_build::build()
}
```

`src-tauri/src/main.rs`:
```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    opencodedeck_lib::run()
}
```

`src-tauri/src/lib.rs`:
```rust
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

### Step 5: Write tauri.conf.json

`src-tauri/tauri.conf.json`:
```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "OpenCodeDeck",
  "version": "0.1.0",
  "identifier": "com.opencodedeck.app",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "windows": [
      {
        "title": "OpenCodeDeck",
        "width": 1000,
        "height": 700,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/32x32.png", "icons/128x128.png", "icons/128x128@2x.png", "icons/icon.icns", "icons/icon.ico"]
  }
}
```

`src-tauri/capabilities/default.json`:
```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Capability for the main window",
  "windows": ["main"],
  "permissions": ["core:default", "shell:allow-open"]
}
```

### Step 6: Add placeholder icons

The tauri.conf.json references icons that must exist for the build to succeed. Generate them. The simplest approach: create a valid PNG (at least 512x512) and use `npx @tauri-apps/cli icon` to generate all required sizes/formats.

```bash
mkdir -p src-tauri/icons
```

Create a valid PNG source (a 512x512 solid-color PNG). If `npx @tauri-apps/cli icon` is available, run:

```bash
npx @tauri-apps/cli icon <source-png> --output src-tauri/icons
```

This generates `32x32.png`, `128x128.png`, `128x128@2x.png`, `icon.icns`, `icon.ico` and others.

If icon generation is problematic, an alternative is to download the Tauri example icons or copy from a Tauri template project. The key requirement: all icon paths referenced in `tauri.conf.json` bundle.icon must exist.

### Step 7: Verify

Run:
```bash
cargo check --manifest-path src-tauri/Cargo.toml
npx tsc --noEmit
```

Expected: both succeed with no errors.

Note: `npm run tauri dev` opens a GUI window which may not be verifiable in a headless environment. The key success criteria are: (1) `cargo check` passes, (2) `npx tsc --noEmit` passes, (3) all files listed above exist with the specified content.

### Step 8: Commit

```bash
git add -A
git commit -m "chore: scaffold Tauri v2 + React + Tailwind project"
```
