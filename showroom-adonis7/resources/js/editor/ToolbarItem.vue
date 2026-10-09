<script setup lang="ts">
import { type Component } from 'vue'

defineProps<{
    isActive?: boolean
    disabled?: boolean
    icon?: Component
    title?: string
}>()
</script>

<template>
    <button
        type="button"
        :disabled="disabled"
        class="p-2 hover:bg-surface-hover transition-colors flex items-center justify-center"
        :class="{ 'bg-surface-hover text-primary': isActive, 'opacity-50 cursor-not-allowed': disabled }"
        :title="title"
    >
        <component :is="icon" v-if="icon" class="w-4 h-4" />
        <slot v-else />
    </button>
</template>
