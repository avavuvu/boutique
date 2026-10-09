<script setup lang="ts">
import { Editor } from '@tiptap/vue-3'
import { computed, useTemplateRef } from 'vue'
import ToolbarItem from './ToolbarItem.vue'
import ToolbarDropdown, { type ToolbarDropdownItem } from './ToolbarDropdown.vue'
import {
    Bold,
    Italic,
    Strikethrough,
    Underline,
    Link as LinkIcon,
    List,
    ListOrdered,
    Undo,
    Redo,
    Heading1,
    Heading2,
    Heading3,
    Pilcrow,
    Quote,
    TextQuote,
} from 'lucide-vue-next'
import LinkMenu from './link/LinkMenu.vue'

const { editor, variant = 'standard' } = defineProps<{
    editor: Editor
    variant?: 'standard' | 'minimal'
}>()

const linkMenu = useTemplateRef('link-menu')

const isBold = computed(() => editor?.isActive('bold') ?? false)
const isItalic = computed(() => editor?.isActive('italic') ?? false)
const isStrike = computed(() => editor?.isActive('strike') ?? false)
const isUnderline = computed(() => editor?.isActive('underline') ?? false)
const isLink = computed(() => editor?.isActive('link') ?? false)
const isBulletList = computed(() => editor?.isActive('bulletList') ?? false)
const isOrderedList = computed(() => editor?.isActive('orderedList') ?? false)

const activeHeading = computed(() => {
    if (editor?.isActive('heading', { level: 1 })) return 'Title'
    if (editor?.isActive('heading', { level: 2 })) return 'Subtitle 1'
    if (editor?.isActive('heading', { level: 3 })) return 'Subtitle 2'
    return 'Normal'
})

const activeQuote = computed(() => {
    if (editor?.isActive('blockquote', { class: 'pullquote' })) return 'Pull Quote'
    if (editor?.isActive('blockquote', { class: 'stdquote' })) return 'Block Quote'
    return null
})

const headingItems = computed<ToolbarDropdownItem[]>(() => [
    {
        id: 'paragraph',
        label: 'Normal Text',
        icon: Pilcrow,
        action: () => editor.chain().focus().setParagraph().run(),
        isActive: editor.isActive('paragraph'),
    },
    {
        id: 'h1',
        label: 'Title',
        icon: Heading1,
        action: () => editor.chain().focus().toggleHeading({ level: 1 }).run(),
        isActive: editor.isActive('heading', { level: 1 }),
    },
    {
        id: 'h2',
        label: 'Subtitle 1',
        icon: Heading2,
        action: () => editor.chain().focus().toggleHeading({ level: 2 }).run(),
        isActive: editor.isActive('heading', { level: 2 }),
    },
    {
        id: 'h3',
        label: 'Subtitle 2',
        icon: Heading3,
        action: () => editor.chain().focus().toggleHeading({ level: 3 }).run(),
        isActive: editor.isActive('heading', { level: 3 }),
    },
])

const quoteItems = computed<ToolbarDropdownItem[]>(() => [
    {
        id: 'blockquote',
        label: 'Block Quote',
        icon: Quote,
        action: () => {
            const isActive = editor.isActive('blockquote', { class: 'stdquote' })
            if (isActive) {
                return editor.chain().focus().unsetBlockquote().run()
            }
            if (editor.isActive('blockquote')) {
                return editor
                    .chain()
                    .focus()
                    .updateAttributes('blockquote', { class: 'stdquote' })
                    .run()
            }
            return editor
                .chain()
                .focus()
                .setBlockquote()
                .updateAttributes('blockquote', { class: 'stdquote' })
                .run()
        },
        isActive: editor.isActive('blockquote', { class: 'stdquote' }),
    },
    {
        id: 'pullquote',
        label: 'Pull Quote',
        icon: TextQuote,
        action: () => {
            const isActive = editor.isActive('blockquote', { class: 'pullquote' })
            if (isActive) {
                return editor.chain().focus().unsetBlockquote().run()
            }
            if (editor.isActive('blockquote')) {
                return editor
                    .chain()
                    .focus()
                    .updateAttributes('blockquote', { class: 'pullquote' })
                    .run()
            }
            return editor
                .chain()
                .focus()
                .setBlockquote()
                .updateAttributes('blockquote', { class: 'pullquote' })
                .run()
        },
        isActive: editor.isActive('blockquote', { class: 'pullquote' }),
    },
])

const setLink = () => {
    linkMenu.value?.setLink()
}
</script>

<template>
    <LinkMenu :editor ref="link-menu" />

    <div v-if="editor" class="flex items-center gap-1 px-2 sticky top-0 z-10">
        <!-- Undo / Redo -->
        <div class="flex items-center gap-0.5 pr-2 border-r border-border">
            <ToolbarItem
                :icon="Undo"
                :disabled="!editor.can().undo()"
                title="Undo"
                @click="editor.chain().focus().undo().run()"
            />
            <ToolbarItem
                :icon="Redo"
                :disabled="!editor.can().redo()"
                title="Redo"
                @click="editor.chain().focus().redo().run()"
            />
        </div>

        <!-- Headings -->
        <div v-if="variant === 'standard'" class="px-2 border-r border-border flex gap-1">
            <ToolbarDropdown class="w-24" :items="headingItems" :label="activeHeading" />
            <ToolbarDropdown :items="quoteItems" :icon="Quote" :is-active="!!activeQuote" />
        </div>

        <!-- Formatting -->
        <div class="flex items-center gap-0.5 px-2 border-r border-border">
            <ToolbarItem
                :icon="Bold"
                :is-active="isBold"
                title="Bold"
                @click="editor.chain().focus().toggleBold().run()"
            />
            <ToolbarItem
                :icon="Strikethrough"
                :is-active="isStrike"
                title="Strike"
                @click="editor.chain().focus().toggleStrike().run()"
            />
            <ToolbarItem
                :icon="Underline"
                :is-active="isUnderline"
                title="Underline"
                @click="editor.chain().focus().toggleUnderline().run()"
            />
            <ToolbarItem
                :icon="Italic"
                :is-active="isItalic"
                title="Italic"
                @click="editor.chain().focus().toggleItalic().run()"
            />
        </div>

        <!-- Link -->
        <div
            v-if="variant === 'standard'"
            class="flex items-center gap-0.5 px-2 border-r border-border"
        >
            <ToolbarItem :icon="LinkIcon" :is-active="isLink" title="Link" @click="setLink" />
        </div>

        <!-- Lists -->
        <div v-if="variant === 'standard'" class="flex items-center gap-0.5 pl-2">
            <ToolbarItem
                :icon="List"
                :is-active="isBulletList"
                title="Bullet List"
                @click="editor.chain().focus().toggleBulletList().run()"
            />
            <ToolbarItem
                :icon="ListOrdered"
                :is-active="isOrderedList"
                title="Ordered List"
                @click="editor.chain().focus().toggleOrderedList().run()"
            />
        </div>
    </div>
</template>
