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

export type SetupContext = { signal: AbortSignal };

export type Use = { event: string; tag: string; target?: "window" | "document" };

type EventIn<Map, Name> = Name extends keyof Map ? Map[Name] : Event;

export type EventOf<U extends Use> = U extends { target: "window" }
    ? EventIn<WindowEventMap, U["event"]>
    : U extends { target: "document" }
      ? EventIn<DocumentEventMap, U["event"]>
      : EventIn<HTMLElementEventMap, U["event"]>;

export type Handler<U extends Use> = (event: EventOf<U>, element: ElementOf<U["tag"]>) => unknown;

export type Handlers<On> = { [Name in keyof On]: On[Name] extends Use ? Handler<On[Name]> : never };

type Lifecycle = { disconnect?: () => unknown };

type Returned<On> = [keyof On] extends [never] ? void | Lifecycle : Handlers<On> & Lifecycle;

export type Setup<Tag extends string, Shape = {}, On = {}> = (
    element: ElementOf<Tag>,
    refs: Refs<Shape>,
    context: SetupContext,
) => Returned<On> | Promise<Returned<On>>;

type AnySetup = (element: any, refs: any, context: any) => unknown;

type Entry = { tags: readonly string[]; many: readonly string[] } & (
    | { setup: AnySetup }
    | { load: () => Promise<AnySetup> }
);

export type Registry = Record<string, Entry>;

type Instance = {
    name: string;
    controller: AbortController;
    ready: boolean;
    settled: boolean;
    handlers?: Record<string, unknown> | undefined;
    bound: WeakMap<Element, Set<string>>;
};

type Binding = { key: string; text: string; event: string; modifiers: Set<string>; setup?: string | undefined; handler?: string };

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

declare const VALUE: unique symbol;

export type ContextKey<T> = { readonly name: string; readonly [VALUE]?: T };

type Waiting = { element: Element; key: ContextKey<unknown>; resolve: (value: unknown) => void; reject: (error: Error) => void };

let defined = false;
const running = new WeakMap<Element, Map<string, Instance>>();
const provided = new WeakMap<Element, Map<ContextKey<unknown>, unknown>>();
const waiting = new Set<Waiting>();

export function context<T>(name: string): ContextKey<T> {
    return Object.freeze({ name });
}

export function provide<T>(element: Element, key: ContextKey<T>, value: T): void {
    if (!element.hasAttribute(ATTRIBUTE)) console.warn(`[bq] provide("${key.name}") needs an element with bq-setup`, element);
    let values = provided.get(element);
    if (!values) {
        values = new Map();
        provided.set(element, values);
    }
    values.set(key, value);
    check();
}

export function inject<T>(element: Element, key: ContextKey<T>): Promise<T> {
    return new Promise<T>((resolve, reject) => {
        const entry: Waiting = { element, key, resolve: resolve as (value: unknown) => void, reject };
        if (!attempt(entry)) waiting.add(entry);
    });
}

function attempt(entry: Waiting): boolean {
    if (!entry.element.isConnected) {
        entry.reject(new DOMException(`[bq] the element was removed before "${entry.key.name}" was provided`, "AbortError"));
        return true;
    }
    for (let owner = entry.element.parentElement?.closest(`[${ATTRIBUTE}]`); owner; owner = owner.parentElement?.closest(`[${ATTRIBUTE}]`)) {
        const values = provided.get(owner);
        if (values?.has(entry.key)) {
            entry.resolve(values.get(entry.key));
            return true;
        }
        const instances = running.get(owner);
        if (namesOf(owner).some((name) => !instances?.get(name)?.settled)) return false;
    }
    entry.reject(new Error(`[bq] no setup above this element provides "${entry.key.name}"`));
    return true;
}

function check(): void {
    for (const entry of waiting) {
        if (attempt(entry)) waiting.delete(entry);
    }
}

function settle(instance: Instance): void {
    instance.settled = true;
    check();
}

function camel(name: string): string {
    return name.replace(/-([a-z0-9])/g, (_, next: string) => next.toUpperCase());
}

function namesOf(element: Element): string[] {
    return (element.getAttribute(ATTRIBUTE) ?? "").split(/\s+/).filter(Boolean);
}

function split(token: string): { setup?: string; name: string } {
    const dot = token.indexOf(".");
    return dot < 0 ? { name: camel(token) } : { setup: token.slice(0, dot), name: camel(token.slice(dot + 1)) };
}

function parseBindings(attribute: string, value: string): Binding[] {
    const [event = "", ...rest] = attribute.slice(ON.length).split("|");
    const modifiers = new Set(rest);
    const tokens = value.split(/\s+/).filter(Boolean);
    if (tokens.length === 0) return [{ key: attribute, text: attribute, event, modifiers }];
    return tokens.map((token) => {
        const { setup, name } = split(token);
        return { key: `${attribute}=${token}`, text: `${attribute}="${token}"`, event, modifiers, setup, handler: name };
    });
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

    const loads = new Map<string, Promise<AnySetup>>();
    const warned = new WeakMap<Element, Set<string>>();

    const warnOnce = (element: Element, key: string, message: string): void => {
        let keys = warned.get(element);
        if (!keys) {
            keys = new Set();
            warned.set(element, keys);
        }
        if (keys.has(key)) return;
        keys.add(key);
        console.warn(message, element);
    };

    const ownerOf = (element: Element, start: Element | null, setup: string | undefined, text: string): [Element, string] | undefined => {
        if (setup !== undefined) {
            for (let owner = start?.closest(`[${ATTRIBUTE}]`); owner; owner = owner.parentElement?.closest(`[${ATTRIBUTE}]`)) {
                if (namesOf(owner).includes(setup)) return [owner, setup];
            }
            warnOnce(element, text, `[bq] ${text} is not inside an element with bq-setup="${setup}"`);
            return undefined;
        }
        const owner = start?.closest(`[${ATTRIBUTE}]`);
        const names = owner ? namesOf(owner) : [];
        const [only] = names;
        if (owner && names.length === 1 && only !== undefined) return [owner, only];
        warnOnce(
            element,
            text,
            owner
                ? `[bq] ${text} is inside more than one setup (${names.join(", ")}); write the setup name, for example "${names[0]}.name"`
                : `[bq] ${text} is not inside an element with bq-setup`,
        );
        return undefined;
    };

    const refsOf = (root: Element, setup: string, entry: Entry): Record<string, Element | Element[]> => {
        const refs: Record<string, Element | Element[]> = {};
        for (const name of entry.many) refs[name] = [];

        for (const element of root.querySelectorAll(`[${REF}]`)) {
            for (const token of (element.getAttribute(REF) ?? "").split(/\s+/).filter(Boolean)) {
                const { setup: prefix, name } = split(token);
                const owner = ownerOf(element, element.parentElement, prefix, `bq-ref="${token}"`);
                if (owner?.[0] !== root || owner[1] !== setup) continue;

                const existing = refs[name];
                if (Array.isArray(existing)) existing.push(element);
                else if (existing === undefined) refs[name] = element;
            }
        }

        return refs;
    };

    const attach = (instance: Instance, element: Element, binding: Binding): void => {
        let bound = instance.bound.get(element);
        if (bound?.has(binding.key)) return;
        if (!bound) {
            bound = new Set();
            instance.bound.set(element, bound);
        }
        bound.add(binding.key);

        let handler: Function | undefined;
        if (binding.handler !== undefined) {
            const found = binding.handler === "disconnect" ? undefined : instance.handlers?.[binding.handler];
            if (typeof found !== "function") {
                console.warn(`[bq] setup "${instance.name}" has no handler "${binding.handler}"`, element);
                return;
            }
            handler = found;
        }

        const { modifiers } = binding;
        const target: EventTarget = modifiers.has("window") ? window : modifiers.has("document") ? document : element;
        const capture = modifiers.has("capture");

        const listener = (event: Event): void => {
            if (!passes(event, element, modifiers)) return;
            if (modifiers.has("prevent")) event.preventDefault();
            if (modifiers.has("stop")) event.stopPropagation();
            if (modifiers.has("once")) target.removeEventListener(binding.event, listener, { capture });
            if (!handler) return;
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
                for (const binding of parseBindings(attribute, value)) {
                    const nearest = element.closest(`[${ATTRIBUTE}]`);
                    const owner =
                        binding.handler === undefined
                            ? nearest && ([nearest, namesOf(nearest)[0]] as [Element, string])
                            : ownerOf(element, element, binding.setup, binding.text);
                    if (!owner) continue;
                    const instance = running.get(owner[0])?.get(owner[1]);
                    if (instance?.ready) attach(instance, element, binding);
                }
            }
        }
    };

    const run = (element: Element, entry: Entry, setup: AnySetup, instance: Instance): void => {
        const { signal } = instance.controller;
        if (signal.aborted) return;

        const failed = (error: unknown): void => {
            console.error(`[bq] setup "${instance.name}" failed`, error);
            settle(instance);
        };

        const ready = (handlers: unknown): void => {
            const object = handlers && typeof handlers === "object" ? (handlers as Record<string, unknown>) : undefined;
            const disconnect = object?.disconnect;
            const stop = (): void => {
                if (typeof disconnect !== "function") return;
                try {
                    const result: unknown = disconnect.call(object);
                    if (result instanceof Promise) result.catch(failed);
                } catch (error) {
                    failed(error);
                }
            };

            if (signal.aborted) {
                stop();
                return;
            }
            signal.addEventListener("abort", stop, { once: true });
            instance.ready = true;
            instance.handlers = object;
            bindWithin(element);
            settle(instance);
        };

        try {
            const result = setup(element, refsOf(element, instance.name, entry), { signal });
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
            const instance: Instance = { name, controller: new AbortController(), ready: false, settled: false, bound: new WeakMap() };
            instances.set(name, instance);

            const entry = registry[name];
            if (!entry) {
                console.warn(`[bq] there is no setup named "${name}"`, element);
                settle(instance);
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
                    (error: unknown) => {
                        console.error(`[bq] could not load setup "${name}"`, error);
                        settle(instance);
                    },
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
            provided.delete(element);
        }
        check();
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
