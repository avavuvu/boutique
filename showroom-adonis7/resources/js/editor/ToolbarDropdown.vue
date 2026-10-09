<script setup lang="ts">
import { ref, onMounted, onUnmounted, type Component } from 'vue'
import { ChevronDown } from 'lucide-vue-next'

export interface ToolbarDropdownItem {
    id: string
    label: string
    icon?: Component
    action: () => void
    isActive?: boolean
}

defineProps<{
    items: ToolbarDropdownItem[]
    label?: string
    icon?: Component
    isActive?: boolean
}>()

const open = ref(false)
const root = ref<HTMLElement | null>(null)

const toggle = () => (open.value = !open.value)

const select = (item: ToolbarDropdownItem) => {
    item.action()
    open.value = false
}

const onClickOutside = (e: MouseEvent) => {
    if (root.value && !root.value.contains(e.target as Node)) {
        open.value = false
    }
}

const onKeydown = (e: KeyboardEvent) => {
    if (e.key === 'Escape') open.value = false
}

onMounted(() => {
    document.addEventListener('mousedown', onClickOutside)
    document.addEventListener('keydown', onKeydown)
})

onUnmounted(() => {
    document.removeEventListener('mousedown', onClickOutside)
    document.removeEventListener('keydown', onKeydown)
})
</script>

<template>
    <div ref="root" class="relative">
        <button
            type="button"
            class="flex justify-between items-center gap-1 p-2 hover:bg-surface-hover transition-colors text-sm font-medium"
            :class="{ 'bg-surface-hover': isActive || open }"
            @click="toggle"
        >
            <component :is="icon" v-if="icon" class="w-4 h-4" />
            <span v-if="label">{{ label }}</span>
            <ChevronDown class="w-3 h-3 opacity-50" />
        </button>

        <div
            v-if="open"
            class="absolute top-full left-0 min-w-[10rem] bg-surface border border-border shadow-lg p-1 z-50 flex flex-col gap-0.5"
        >
            <button
                v-for="item in items"
                :key="item.id"
                type="button"
                class="flex items-center gap-2 px-2 py-1.5 text-sm hover:bg-surface-hover w-full text-left"
                :class="{ 'bg-surface-hover': item.isActive }"
                @click="select(item)"
            >
                <component :is="item.icon" v-if="item.icon" class="w-4 h-4" />
                <span>{{ item.label }}</span>
            </button>
        </div>
    </div>
</template>
