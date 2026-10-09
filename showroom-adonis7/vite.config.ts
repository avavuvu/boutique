import { defineConfig } from 'vite'
import tailwindcss from '@tailwindcss/vite'
import adonisjs from '@adonisjs/vite/client'
import vue from '@vitejs/plugin-vue'

export default defineConfig({
    plugins: [
        tailwindcss(),
        vue(),
        adonisjs({
            entrypoints: ['resources/css/app.css', 'resources/js/islands.ts'],
            reload: ['resources/views/**/*.edge'],
        }),
    ],

    server: {
        watch: {
            ignored: ['**/storage/**', '**/tmp/**'],
        },
        allowedHosts: ['.localtest.me'],
    },
})
