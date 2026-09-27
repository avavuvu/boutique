const MOUNTED = "data-island-mounted";

function findModule(modules, name) {
    const suffix = new RegExp(`/${name}\\.[a-z]+$`);
    const key = Object.keys(modules).find((k) => suffix.test(k));
    return key ? modules[key] : null;
}

async function mountOne(element, modules, mount) {
    if (element.hasAttribute(MOUNTED)) return;
    element.setAttribute(MOUNTED, "");

    const name = element.getAttribute("data-island");
    const loader = findModule(modules, name);
    if (!loader) {
        console.warn(`[islands] no component found for "${name}"`);
        return;
    }

    const { default: component } = await loader();
    const propsAttr = element.getAttribute("data-props");
    const props = propsAttr ? JSON.parse(propsAttr) : {};

    mount(element, component, props);
}

window.mountIslands = (modules, mount) => {
    const scan = (root) =>
        root.querySelectorAll("[data-island]").forEach((el) => mountOne(el, modules, mount));

    scan(document);
    document.addEventListener("htmx:after:settle", (event) => scan(event.target));
};
