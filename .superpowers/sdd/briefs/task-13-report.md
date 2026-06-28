# Task 13 Report — Setup shadcn/ui with components and theme

## What was done

1. **Installed peer deps** — `clsx`, `tailwind-merge`, `class-variance-authority`, `lucide-react`, `sonner`.
2. **Path alias** — added `"paths": { "@/*": ["./src/*"] }` to `tsconfig.json` compilerOptions. Did NOT add `baseUrl`: it is deprecated in TypeScript 6.0 (errors with TS5101), and `moduleResolution: "bundler"` resolves relative `paths` without it.
3. **Vite alias** — added `resolve.alias` mapping `@` → `path.resolve(__dirname, "./src")` in `vite.config.ts`.
4. **`src/lib/utils.ts`** — created `cn()` helper using `clsx` + `twMerge`.
5. **Theme variables** — rewrote `src/styles/globals.css` with shadcn HSL CSS variables for `:root` and `.dark`, plus base border/bg/text rules. Kept the existing `@import "tailwindcss"` and `@config "../../tailwind.config.ts"` directives (Tailwind v4 CSS-first + v3-style config bridge that the project already used).
6. **`tailwind.config.ts`** — extended theme with shadcn color tokens (`border`, `input`, `ring`, `background`, `foreground`, `primary`, `secondary`, `destructive`, `muted`, `accent`, `card`), `borderRadius` keyed off `--radius`, and `darkMode: ["class"]`.
7. **`components.json`** — created with `new-york` style, `neutral` base color, CSS variables enabled, alias map pointing at `@/components/ui` etc.
8. **shadcn components installed** via CLI — `npx shadcn@latest add button card input switch label tabs dialog sonner badge` succeeded, creating 9 files under `src/components/ui/`. CLI also installed Radix deps (`@radix-ui/react-slot`, `react-switch`, `react-label`, `react-tabs`, `react-dialog`) and `next-themes` (required by the `sonner` component).
9. **Verified** — `npx tsc --noEmit` passes with no errors; `npm run build` succeeds (CSS 27 kB, JS 192 kB).

## shadcn CLI issues

None. The CLI ran cleanly in the non-interactive environment once `components.json` was present. No manual component creation was needed.

## Notes / deviations from spec

- **`baseUrl` omitted.** The spec's `tsconfig.json` step included `"baseUrl": "."`, but with `typescript@^6.0.3` this triggers `error TS5101: Option 'baseUrl' is deprecated`. Removed it; `paths` resolution works via `moduleResolution: "bundler"` + the Vite alias. Verified both `tsc` and `vite build` resolve `@/...` imports correctly.
- Kept the pre-existing `@config "../../tailwind.config.ts"` line in `globals.css` so the v3-style `tailwind.config.ts` (needed for shadcn's color/borderRadius theme keys) is picked up by Tailwind v4's PostCSS plugin.

## Commit

```
dfdaf8da12de6a2960db1e7259b17700159093a7
feat(frontend): setup shadcn/ui with components and theme
```
