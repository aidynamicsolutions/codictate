import { defineConfig, type PluginOption } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { sentryVitePlugin } from "@sentry/vite-plugin";
import { resolve } from "path";
import packageJson from "./package.json";

const host = process.env.TAURI_DEV_HOST;
const sentryRelease =
  process.env.SENTRY_RELEASE || `codictate@${packageJson.version}`;
const hasSentryUploadConfig =
  Boolean(process.env.SENTRY_AUTH_TOKEN) &&
  Boolean(process.env.SENTRY_ORG) &&
  Boolean(process.env.SENTRY_PROJECT);

function normalizePlugins(input: PluginOption): PluginOption[] {
  if (Array.isArray(input)) {
    return input.flatMap(normalizePlugins);
  }
  return input ? [input] : [];
}

// https://vitejs.dev/config/
export default defineConfig(() => {
  const sourcemapMode: false | "hidden" = hasSentryUploadConfig
    ? "hidden"
    : false;

  const plugins: PluginOption[] = [
    ...normalizePlugins(react()),
    ...normalizePlugins(tailwindcss()),
  ];

  if (hasSentryUploadConfig) {
    plugins.push(
      ...normalizePlugins(
        sentryVitePlugin({
          authToken: process.env.SENTRY_AUTH_TOKEN,
          org: process.env.SENTRY_ORG,
          project: process.env.SENTRY_PROJECT,
          release: {
            name: sentryRelease,
          },
          sourcemaps: {
            assets: "./dist/**/*.{js,mjs,map}",
          },
          telemetry: false,
        }),
      ),
    );
  }

  return {
    plugins,

    // Path aliases
    resolve: {
      alias: {
        "@": resolve(__dirname, "./src"),
        "@/bindings": resolve(__dirname, "./src/bindings.ts"),
      },
    },

    // Multiple entry points for main app and overlay
    build: {
      sourcemap: sourcemapMode,
      rollupOptions: {
        input: {
          main: resolve(__dirname, "index.html"),
          overlay: resolve(__dirname, "src/overlay/index.html"),
        },
      },
    },

    // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
    //
    // 1. prevent vite from obscuring rust errors
    clearScreen: false,
    // 2. tauri expects a fixed port, fail if that port is not available
    server: {
      port: 1420,
      strictPort: true,
      host: host || false,
      hmr: host
        ? {
            protocol: "ws",
            host,
            port: 1421,
          }
        : undefined,
      watch: {
        // 3. tell vite to ignore watching `src-tauri`
        ignored: ["**/src-tauri/**"],
      },
    },
  };
});
