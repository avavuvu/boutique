<script setup lang="ts">
import { useEditor, EditorContent } from '@tiptap/vue-3'
import StarterKit from '@tiptap/starter-kit'
import UnderlineExtension from '@tiptap/extension-underline'
import { onMounted, onUnmounted, ref } from 'vue'
import Toolbar from '../editor/Toolbar.vue'

const props = defineProps<{
    newsletterId: string
}>()

const editor = useEditor({
    content: '',
    extensions: [StarterKit, UnderlineExtension],
    editorProps: {
        attributes: {
            class: 'outline-none prose max-w-none',
        },
    },
    onUpdate: (value) => {
        json.value.innerHTML = JSON.stringify(value.transaction.doc.toJSON(), null, 5)
    },
})

const json = ref()

onMounted(async () => {
    const response = await fetch(`/edit/${props.newsletterId}/newsletter`)
    const { newsletter } = await response.json()
    editor.value!.commands.setContent(JSON.parse(newsletter.content))
})

onUnmounted(() => {
    editor.value?.destroy()
})
</script>

<template>
    <div class="">
        <div v-if="editor" class="border-b border-border bg-surface">
            <Toolbar :editor="editor" />
        </div>
        <editor-content :editor="editor" class="min-h-64 p-4" />
    </div>

    <div>
        <code ref="json"> </code>
    </div>
</template>

<style>
.tiptap p.is-editor-empty:first-child::before {
    color: #9ca3af;
    content: attr(data-placeholder);
    float: left;
    height: 0;
    pointer-events: none;
}
</style>
