// Tauri serves the frontend as static files with no Node server, so render this
// as a client-side SPA: prerender the shell, disable SSR.
// See https://v2.tauri.app/start/frontend/sveltekit/
export const prerender = true;
export const ssr = false;
