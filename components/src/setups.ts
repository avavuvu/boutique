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

export type Use = { event: string; tag: string; target?: "window" | "document" };

type EventIn<Map, Name> = Name extends keyof Map ? Map[Name] : Event;

export type EventOf<U extends Use> = U extends { target: "window" }
    ? EventIn<WindowEventMap, U["event"]>
    : U extends { target: "document" }
      ? EventIn<DocumentEventMap, U["event"]>
      : EventIn<HTMLElementEventMap, U["event"]>;

export type Handler<U extends Use> = (event: EventOf<U>, element: ElementOf<U["tag"]>) => unknown;

export type Handlers<On> = { [Name in keyof On]: On[Name] extends Use ? Handler<On[Name]> : never };

type Returned<On> = [keyof On] extends [never] ? void : Handlers<On>;

export type Setup<Tag extends string, Shape = {}, On = {}> = (
    element: ElementOf<Tag>,
    context: Context<Shape>,
) => Returned<On> | Promise<Returned<On>>;

type AnySetup = (element: any, context: any) => unknown;

type Entry = { tags: readonly string[]; many: readonly string[] } & (
    | { setup: AnySetup }
    | { load: () => Promise<AnySetup> }
);

export type Registry = Record<string, Entry>;

type Instance = {
    name: string;
    controller: AbortController;
    ready: boolean;
    handlers?: Record<string, unknown>;
    bound: WeakMap<Element, Set<string>>;
};

type Binding = { event: string; modifiers: Set<string>; setup?: string; handler: string };

const ATTRIBUTE = "bq-setup";
const REF = "bq-ref";
const ON = "bq-on:";

const KEYS: Record<string, readonly string[]> = {
    enter: ["Enter"],
    escape: ["Escape", "Esc"],
    tab: ["Tab"],
    space: [" ", "Spacebar"],
    delete: ["Delete", "Backspace"],
    up: ["ArrowUp"],
    down: ["ArrowDown"],
    left: ["ArrowLeft"],
    right: ["ArrowRight"],
};
const BUTTONS: Record<string, number> = { left: 0, middle: 1, right: 2 };
const SYSTEM = ["ctrl", "alt", "shift", "meta"] as const;

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

function parseBinding(attribute: string, value: string): Binding {
    const [event = "", ...modifiers] = attribute.slice(ON.length).split("|");
    const dot = value.indexOf(".");
    return dot < 0
        ? { event, modifiers: new Set(modifiers), handler: value.trim() }
        : { event, modifiers: new Set(modifiers), setup: value.slice(0, dot).trim(), handler: value.slice(dot + 1).trim() };
}

function ownerOf(element: Element, setup: string | undefined): [Element, string] | undefined {
    let owner = element.closest(`[${ATTRIBUTE}]`);
    if (setup === undefined) {
        if (!owner) return undefined;
        const names = namesOf(owner);
        if (names.length === 1) return [owner, names[0]];
        console.warn(`[bq] this bq-on is inside more than one setup (${names.join(", ")}); write the setup name, for example "${names[0]}.handler"`, element);
        return undefined;
    }
    while (owner) {
        if (namesOf(owner).includes(setup)) return [owner, setup];
        owner = owner.parentElement?.closest(`[${ATTRIBUTE}]`) ?? null;
    }
    return undefined;
}

function isMac(): boolean {
    return /Mac|iPhone|iPad|iPod/.test(navigator.platform || navigator.userAgent);
}

function passes(event: Event, element: Element, modifiers: Set<string>): boolean {
    if (modifiers.has("self") && event.target !== element) return false;

    if (event instanceof KeyboardEvent) {
        const keys = [...modifiers].filter((modifier) => modifier in KEYS || /^[a-z0-9]$/.test(modifier));
        const key = event.key;
        if (keys.length > 0 && !keys.some((modifier) => KEYS[modifier]?.includes(key) || key.toLowerCase() === modifier)) return false;
    } else if (event instanceof MouseEvent) {
        const buttons = [...modifiers].filter((modifier) => modifier in BUTTONS);
        if (buttons.length > 0 && !buttons.some((modifier) => event.button === BUTTONS[modifier])) return false;
    }

    const required = new Set<string>(SYSTEM.filter((key) => modifiers.has(key)));
    if (modifiers.has("mod")) required.add(isMac() ? "meta" : "ctrl");
    if (required.size === 0 && !modifiers.has("exact")) return true;

    const pressed = event as unknown as Record<string, unknown>;
    for (const key of SYSTEM) {
        const down = pressed[`${key}Key`] === true;
        if (required.has(key) && !down) return false;
        if (modifiers.has("exact") && !required.has(key) && down) return false;
    }
    return true;
}

export function defineSetups(registry: Registry): void {
    if (defined) {
        console.warn("[bq] defineSetups was called more than once");
        return;
    }
    defined = true;

    const running = new WeakMap<Element, Map<string, Instance>>();
    const loads = new Map<string, Promise<AnySetup>>();

    const attach = (instance: Instance, element: Element, attribute: string, binding: Binding): void => {
        let bound = instance.bound.get(element);
        if (bound?.has(attribute)) return;
        if (!bound) {
            bound = new Set();
            instance.bound.set(element, bound);
        }
        bound.add(attribute);

        const handler = instance.handlers?.[binding.handler];
        if (typeof handler !== "function") {
            console.warn(`[bq] setup "${instance.name}" has no handler "${binding.handler}"`, element);
            return;
        }

        const { modifiers } = binding;
        const target: EventTarget = modifiers.has("window") ? window : modifiers.has("document") ? document : element;
        const capture = modifiers.has("capture");

        const listener = (event: Event): void => {
            if (!passes(event, element, modifiers)) return;
            if (modifiers.has("prevent")) event.preventDefault();
            if (modifiers.has("stop")) event.stopPropagation();
            if (modifiers.has("once")) target.removeEventListener(binding.event, listener, { capture });
            try {
                const result: unknown = handler.call(instance.handlers, event, element);
                if (result instanceof Promise) {
                    result.catch((error: unknown) => console.error(`[bq] handler "${instance.name}.${binding.handler}" failed`, error));
                }
            } catch (error) {
                console.error(`[bq] handler "${instance.name}.${binding.handler}" failed`, error);
            }
        };

        target.addEventListener(binding.event, listener, { signal: instance.controller.signal, capture, passive: modifiers.has("passive") });
    };

    const bindWithin = (root: Element): void => {
        for (const element of [root, ...root.querySelectorAll("*")]) {
            for (const { name: attribute, value } of [...element.attributes]) {
                if (!attribute.startsWith(ON)) continue;
                const binding = parseBinding(attribute, value);
                const owner = ownerOf(element, binding.setup);
                if (!owner) continue;
                const instance = running.get(owner[0])?.get(owner[1]);
                if (instance?.ready) attach(instance, element, attribute, binding);
            }
        }
    };

    const run = (element: Element, entry: Entry, setup: AnySetup, instance: Instance): void => {
        const { signal } = instance.controller;
        if (signal.aborted) return;

        const ready = (handlers: unknown): void => {
            if (signal.aborted) return;
            instance.ready = true;
            if (handlers && typeof handlers === "object") instance.handlers = handlers as Record<string, unknown>;
            bindWithin(element);
        };
        const failed = (error: unknown): void => console.error(`[bq] setup "${instance.name}" failed`, error);

        try {
            const result = setup(element, { ...refsOf(element, entry.many), signal });
            if (result instanceof Promise) result.then(ready, failed);
            else ready(result);
        } catch (error) {
            failed(error);
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
        let instances = running.get(element);

        if (instances) {
            for (const [name, instance] of instances) {
                if (names.includes(name)) continue;
                instance.controller.abort();
                instances.delete(name);
            }
        }

        for (const name of names) {
            if (instances?.has(name)) continue;

            if (!instances) {
                instances = new Map();
                running.set(element, instances);
            }
            const instance: Instance = { name, controller: new AbortController(), ready: false, bound: new WeakMap() };
            instances.set(name, instance);

            const entry = registry[name];
            if (!entry) {
                console.warn(`[bq] there is no setup named "${name}"`, element);
                continue;
            }

            if (entry.tags.length > 0 && !entry.tags.includes(element.localName)) {
                console.warn(`[bq] setup "${name}" expects <${entry.tags.join("> or <")}>, not <${element.localName}>`, element);
            }

            if ("setup" in entry) {
                run(element, entry, entry.setup, instance);
            } else {
                load(name, entry.load).then(
                    (setup) => run(element, entry, setup, instance),
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
            for (const instance of running.get(element)?.values() ?? []) instance.controller.abort();
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
                if (!(node instanceof Element) || !node.isConnected) return;
                scan(node);
                bindWithin(node);
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
