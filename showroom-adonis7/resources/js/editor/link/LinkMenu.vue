<script setup lang="ts">
import { ref } from 'vue';
import { CheckIcon, Trash, Link, ExternalLink } from 'lucide-vue-next';
import { getMarkRange, type Editor } from '@tiptap/vue-3';

const props = defineProps<{ editor: Editor }>();

const showLinkMenu = ref(false);
const linkUrl = ref('');
const linkDisplay = ref('');
const linkMenuPosition = ref({
    top: '0px',
    left: '0px',
    transform: 'translateX(-50%)'
});

const setLink = () => {
    const { view, state } = props.editor;
    const { from, to, $from, } = state.selection;
    let start = view.coordsAtPos(from);
    let end = view.coordsAtPos(to);
    linkDisplay.value = state.doc.textBetween(from, to, '')

    const range = getMarkRange($from, state.schema.marks.link)
    if (range) {
        linkDisplay.value = state.doc.textBetween(range.from, range.to, '')
        start = view.coordsAtPos(range.from);
        end = view.coordsAtPos(range.to);
    }

    // Position the menu below the selection
    linkMenuPosition.value = {
        top: `${end.bottom + 10}px`,
        left: `${(start.left + end.left) / 2}px`,
        transform: 'translateX(-50%)'
    };

    showLinkMenu.value = true;

    const previousUrl = props.editor.getAttributes('link').href;
    linkUrl.value = previousUrl || '';

};

const saveLink = () => {
    console.log(linkUrl.value)

    if (linkUrl.value === '') {
        removeLink();
        return;
    }

    const normalizedUrl = normalizeUrl(linkUrl.value)

    const { state } = props.editor;
    const { $from } = state.selection

    // this is assuming there is already a link for us to select its mark
    const range = getMarkRange($from, state.schema.marks.link)

    // if there is no range, use the regular selection
    let { from, to } = range ? {
        from: range.from,
        to: range.to,
    } : state.selection

    props.editor
        .chain()
        .focus()
        .deleteRange({ from, to })
        .insertContent(linkDisplay.value)
        .run();

    const newTo = from + linkDisplay.value.length;
    props.editor
        .chain()
        .focus()
        .setTextSelection({ from, to: newTo })
        .setLink({ href: normalizedUrl })
        .run();

    closeLinkMenu();
};

const removeLink = () => {
    props.editor
        .chain()
        .focus()
        .extendMarkRange('link')
        .unsetLink()
        .run();

    closeLinkMenu();
};

const closeLinkMenu = () => {
    showLinkMenu.value = false;
    linkUrl.value = '';
};

const normalizeUrl = (url: string) => {
    if (!url || !url.trim()) return '';

    let normalizedUrl = url.trim();

    if (!/^https?:\/\//i.test(normalizedUrl)) {
        normalizedUrl = 'https://' + normalizedUrl;
    }

    try {
        const urlObject = new URL(normalizedUrl);
        return urlObject.href;
    } catch (e) {
        console.warn('Invalid URL:', url);
        return url;
    }
};

defineExpose({
    setLink
});

props.editor.on("selectionUpdate", ({ transaction }) => {
    const linkMark = transaction.doc.nodeAt(transaction.selection.from)?.marks.find(mark => mark.type.name === 'link');

    if (linkMark) {
        setLink()
    } else if (showLinkMenu.value) {
        closeLinkMenu()
    }
})

</script>

<template>
    <div v-if="showLinkMenu" ref="link-menu" class="
        fixed z-100 p-2 items-center bg-surface border shadow
        grid grid-rows-3 gap-1" :style="linkMenuPosition">
        <input ref="text-display-input" class="outline-none border px-2 border-dashed" v-model="linkDisplay" type="text"
            placeholder="Display as..." @keydown.enter="saveLink" @keydown.escape="closeLinkMenu" />
        <div class="inline-flex justify-between border items-center px-2">
            <input ref="link-input" class="outline-none underline" v-model="linkUrl" type="text"
                placeholder="Enter URL..." @keydown.enter="saveLink" @keydown.escape="closeLinkMenu" />
            <a :href="linkUrl" target="_blank">
                <ExternalLink class="w-4" />
            </a>
        </div>
        <div class="flex justify-end">
            <button @click="saveLink" class="cursor-pointer hover:bg-surface-subtle">
                <CheckIcon />
            </button>
            <button @click="removeLink" class="cursor-pointer hover:bg-surface-subtle">
                <Trash />
            </button>

        </div>
    </div>
</template>
