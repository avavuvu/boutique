import { fileURLToPath } from "node:url"
import { defineConfig } from "vite"

export const boutique = (input) => defineConfig({
    publicDir: false,
    base: "/build/",
    resolve: {
        alias: {
            "@setups": fileURLToPath(new URL("../../setups.ts", import.meta.url)),
        },
    },
    build: {
        target: ["chrome114", "edge114", "firefox125", "safari17"],
        outDir: "public/build",
        emptyOutDir: true,
        manifest: true,
        rollupOptions: {
            input,
            output: {
                entryFileNames: "[name]-[hash].js",
                chunkFileNames: "[name]-[hash].js",
                assetFileNames: "[name]-[hash].[ext]",
            },
        },
    },
})
