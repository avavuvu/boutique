export type ElementOf<Tag extends string> = Tag extends keyof HTMLElementTagNameMap
    ? HTMLElementTagNameMap[Tag]
    : Tag extends keyof SVGElementTagNameMap
      ? SVGElementTagNameMap[Tag]
      : HTMLElement;

type Resolve<Value> = Value extends readonly (infer Tag extends string)[]
    ? ElementOf<Tag>[]
    : Value extends string
      ? ElementOf<Value>
      : never;

export type Refs<Shape> = { [Key in keyof Shape]: Resolve<NonNullable<Shape[Key]>> };

export type Context<Shape = {}> = Refs<Shape> & { signal: AbortSignal };

export type Setup<Tag extends string, Shape = {}> = (
    element: ElementOf<Tag>,
    context: Context<Shape>,
) => void | Promise<void>;

type AnySetup = (element: any, context: any) => void | Promise<void>;

type Entry = { tags: readonly string[]; many: readonly string[] } & (
    | { setup: AnySetup }
    | { load: () => Promise<AnySetup> }
);

export type Registry = Record<string, Entry>;

const ATTRIBUTE = "bq-setup";
const REF = "bq-ref";

let defined = false;

function camel(name: string): string {
    return name.replace(/-([a-z0-9])/g, (_, next: string) => next.toUpperCase());
}

function namesOf(element: Element): string[] {
    return (element.getAttribute(ATTRIBUTE) ?? "").split(/\s+/).filter(Boolean);
}

function refsOf(root: Element, many: readonly string[]): Record<string, Element | Element[]> {
    const refs: Record<string, Element | Element[]> = {};
    for (const name of many) refs[name] = [];

    for (const element of root.querySelectorAll(`[${REF}]`)) {
        if (element.parentElement?.closest(`[${ATTRIBUTE}]`) !== root) continue;

        const name = camel(element.getAttribute(REF) ?? "");
        const existing = refs[name];
        if (Array.isArray(existing)) existing.push(element);
        else if (existing === undefined) refs[name] = element;
    }

    return refs;
}

export function defineSetups(registry: Registry): void {
    if (defined) {
        console.warn("[bq] defineSetups was called more than once");
        return;
    }
    defined = true;

    const running = new WeakMap<Element, Map<string, AbortController>>();
    const loads = new Map<string, Promise<AnySetup>>();

    const run = (element: Element, name: string, entry: Entry, setup: AnySetup, signal: AbortSignal): void => {
        if (signal.aborted) return;
        try {
            const result = setup(element, { ...refsOf(element, entry.many), signal });
            if (result instanceof Promise) result.catch((error: unknown) => console.error(`[bq] setup "${name}" failed`, error));
        } catch (error) {
            console.error(`[bq] setup "${name}" failed`, error);
        }
    };

    const load = (name: string, loader: () => Promise<AnySetup>): Promise<AnySetup> => {
        let promise = loads.get(name);
        if (!promise) {
            promise = loader();
            loads.set(name, promise);
        }
        return promise;
    };

    const connect = (element: Element): void => {
        const names = namesOf(element);
        let controllers = running.get(element);

        if (controllers) {
            for (const [name, controller] of controllers) {
                if (names.includes(name)) continue;
                controller.abort();
                controllers.delete(name);
            }
        }

        for (const name of names) {
            if (controllers?.has(name)) continue;

            if (!controllers) {
                controllers = new Map();
                running.set(element, controllers);
            }
            const controller = new AbortController();
            controllers.set(name, controller);

            const entry = registry[name];
            if (!entry) {
                console.warn(`[bq] there is no setup named "${name}"`, element);
                continue;
            }

            if (entry.tags.length > 0 && !entry.tags.includes(element.localName)) {
                console.warn(`[bq] setup "${name}" expects <${entry.tags.join("> or <")}>, not <${element.localName}>`, element);
            }

            if ("setup" in entry) {
                run(element, name, entry, entry.setup, controller.signal);
            } else {
                load(name, entry.load).then(
                    (setup) => run(element, name, entry, setup, controller.signal),
                    (error: unknown) => console.error(`[bq] could not load setup "${name}"`, error),
                );
            }
        }
    };

    const scan = (root: Element): void => {
        if (root.hasAttribute(ATTRIBUTE)) connect(root);
        root.querySelectorAll(`[${ATTRIBUTE}]`).forEach(connect);
    };

    const disconnect = (root: Element): void => {
        const elements = root.hasAttribute(ATTRIBUTE) ? [root] : [];
        elements.push(...root.querySelectorAll(`[${ATTRIBUTE}]`));
        for (const element of elements) {
            if (element.isConnected) continue;
            for (const controller of running.get(element)?.values() ?? []) controller.abort();
            running.delete(element);
        }
    };

    const observer = new MutationObserver((records) => {
        for (const record of records) {
            if (record.type === "attributes") {
                const target = record.target as Element;
                if (target.isConnected) connect(target);
                continue;
            }
            record.removedNodes.forEach((node) => {
                if (node instanceof Element) disconnect(node);
            });
            record.addedNodes.forEach((node) => {
                if (node instanceof Element && node.isConnected) scan(node);
            });
        }
    });

    observer.observe(document.documentElement, {
        childList: true,
        subtree: true,
        attributes: true,
        attributeFilter: [ATTRIBUTE],
    });
    scan(document.documentElement);
}
