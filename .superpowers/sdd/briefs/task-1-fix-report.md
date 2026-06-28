# Task 1 — Tailwind v4 Config Migration Fix Report

## Status: DONE

## Problem
Project scaffolded with Tailwind CSS v4 (`tailwindcss: ^4.3.1`) but configured v3-style:
- `src/styles/globals.css` used v3 `@tailwind` directives
- `postcss.config.js` used `tailwindcss: {}` (v3) — fails in v4 with: "PostCSS plugin has moved to a separate package"
- `tailwind.config.ts` used v3 `Config` type

## Changes Made

### 1. `package.json` / `package-lock.json`
Installed `@tailwindcss/postcss` as a dev dependency:
```
npm install -D @tailwindcss/postcss
```
Output: `added 16 packages, and audited 54 packages in 2s ... found 0 vulnerabilities`

### 2. `postcss.config.js`
Replaced v3 plugin (`tailwindcss: {}`) with v4 plugin and removed `autoprefixer` (v4 handles vendor prefixes automatically):
```js
export default {
  plugins: { "@tailwindcss/postcss": {} },
}
```

### 3. `src/styles/globals.css`
Replaced v3 directives with v4 CSS-first import, kept `tailwind.config.ts` loaded via `@config`:
```css
@import "tailwindcss";
@config "../../tailwind.config.ts";
```

### 4. `tailwind.config.ts`
Unchanged (per task instructions; loaded via `@config` directive in CSS).

## Verification

### `npx tsc --noEmit`
Passed with no output (no type errors).

### `npm run build` (after fix)
```
> tsc && vite build
vite v8.1.0 building client environment for production...
transforming...✓ 16 modules transformed.
✗ Build failed in 413ms
error during build:
[plugin vite:esbuild-transpile]
Error: Failed to load `transformWithEsbuild`. ... Cannot find package 'esbuild'
```

**Analysis:** The Tailwind/PostCSS error is resolved — all 16 modules (including CSS) transformed successfully. The remaining `esbuild` error is a **pre-existing, unrelated** Vite 8/rolldown issue caused by `vite.config.ts:9` specifying `minify: "esbuild"` without `esbuild` installed. This is out of scope for this task.

### Confirmation that fix resolved the original error
Stashed the fix and re-ran the build on the original code — it failed at the PostCSS step with exactly the error this task targets:
```
Error: [postcss] It looks like you're trying to use `tailwindcss` directly as a PostCSS plugin.
The PostCSS plugin has moved to a separate package, so to continue using Tailwind CSS with
PostCSS you'll need to install `@tailwindcss/postcss` and update your PostCSS configuration.
```
After restoring the fix, that error no longer occurs (CSS transforms cleanly).

## Commit
```
fix: migrate Tailwind config from v3 to v4 style
```
Commit hash: e863f40339bdc69655a75d0161e191d0a9a06120
