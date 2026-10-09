import { configApp } from '@adonisjs/eslint-config'
import pluginVue from 'eslint-plugin-vue'

export default [
    ...configApp(),

    // TypeScript files in resources/js (client-side, not server)
    {
        files: ['resources/js/**/*.ts'],
    },

    // Vue islands
    ...pluginVue.configs['flat/essential'].map((config) => ({
        ...config,
        files: ['resources/js/**/*.vue'],
    })),
    {
        files: ['resources/js/**/*.vue'],
        languageOptions: {
            parserOptions: {
                parser: '@typescript-eslint/parser',
            },
        },
        rules: {
            // Island components are keyed by filename — single-word names are intentional
            'vue/multi-word-component-names': 'off',
        },
    },
]
