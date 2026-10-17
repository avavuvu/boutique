import { afterEach, expect, test } from "bun:test";
import { context, defineSetups, inject, provide } from "../../assets/files/setups.ts";

const until = async (predicate: () => boolean): Promise<void> => {
    for (let i = 0; i < 100; i++) {
        if (predicate()) return;
        await new Promise((resolve) => setTimeout(resolve, 5));
    }
    throw new Error("timed out");
};

const fragment = (html: string): Element => {
    const template = document.createElement("template");
    template.innerHTML = html;
    return template.content.firstElementChild!.cloneNode(true) as Element;
};

const presses: Element[] = [];
const handle = context<{ id: string }>("handle");
const injected: Record<string, unknown> = {};
let loads = 0;
let releaseLoad!: () => void;
const loaded = new Promise<void>((resolve) => (releaseLoad = resolve));
const lazyRuns: Element[] = [];
const lifecycle: { signal?: AbortSignal; disconnected: number } = { disconnected: 0 };

defineSetups({
    "press-counter": {
        tags: [],
        many: [],
        setup: () => ({ press: (_event: Event, element: Element) => presses.push(element) }),
    },
    "self-injector": {
        tags: [],
        many: [],
        setup: (element: Element) => {
            inject(element, handle).then(
                (value) => (injected.self = value),
                (error) => (injected.self = error),
            );
        },
    },
    "child-injector": {
        tags: [],
        many: [],
        setup: (element: Element) => {
            inject(element, handle).then(
                (value) => (injected.child = value),
                (error) => (injected.child = error),
            );
        },
    },
    lazy: {
        tags: [],
        many: [],
        load: async () => {
            loads++;
            await loaded;
            return (element: Element) => {
                lazyRuns.push(element);
            };
        },
    },
    lifecycle: {
        tags: [],
        many: [],
        setup: (_element: Element, _refs: unknown, { signal }: { signal: AbortSignal }) => {
            lifecycle.signal = signal;
            return { disconnect: () => lifecycle.disconnected++ };
        },
    },
});

afterEach(() => {
    document.body.replaceChildren();
});

test("a setup attaches inside a contenteditable container and its handlers fire", async () => {
    const editor = document.createElement("div");
    editor.setAttribute("contenteditable", "true");
    document.body.append(editor);

    const button = fragment(`<button bq-setup="press-counter" bq-on:click="press">Press</button>`) as HTMLButtonElement;
    editor.append(button);

    await until(() => {
        button.click();
        return presses.length > 0;
    });
    expect(presses).toEqual([button]);
});

test("a value provided on a detached root is injectable by the root's own setup and its descendants", async () => {
    const root = fragment(`<section bq-setup="self-injector"><div bq-setup="child-injector"></div></section>`);
    const value = { id: "music" };
    provide(root, handle, value);

    document.body.append(root);

    await until(() => "self" in injected && "child" in injected);
    expect(injected.self).toBe(value);
    expect(injected.child).toBe(value);
});

test("a lazily loaded setup attaches asynchronously, after its module loads", async () => {
    const element = fragment(`<div bq-setup="lazy"></div>`);
    document.body.append(element);
    expect(lazyRuns).toEqual([]);

    await until(() => loads > 0);
    expect(lazyRuns).toEqual([]);

    releaseLoad();
    await until(() => lazyRuns.length > 0);
    expect(lazyRuns).toEqual([element]);
});

test("removing the element runs disconnect and aborts the signal", async () => {
    const element = fragment(`<div bq-setup="lifecycle"></div>`);
    document.body.append(element);
    await until(() => lifecycle.signal !== undefined);
    expect(lifecycle.signal!.aborted).toBe(false);
    expect(lifecycle.disconnected).toBe(0);

    element.remove();

    await until(() => lifecycle.disconnected > 0);
    expect(lifecycle.disconnected).toBe(1);
    expect(lifecycle.signal!.aborted).toBe(true);
});
