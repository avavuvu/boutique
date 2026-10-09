import { createApp, type Component } from 'vue'

// Auto-discover all .vue files under islands/
const modules = import.meta.glob<{ default: Component }>('./islands/*.vue')

document.querySelectorAll<HTMLElement>('[data-island]').forEach(async (element) => {
    const name = element.getAttribute('data-island')
    if (!name) return

    const loader = modules[`./islands/${name}.vue`]
    if (!loader) {
        console.warn(`[islands] No component found for: "${name}"`)
        return
    }

    const { default: Component } = await loader()

    const propsAttr = element.getAttribute('data-props')
    const props = propsAttr ? JSON.parse(propsAttr) : {}

    createApp(Component, props).mount(element)
})
