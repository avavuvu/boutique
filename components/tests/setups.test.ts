import assert from "node:assert/strict";
import { afterEach, before, test } from "node:test";
import { JSDOM } from "jsdom";

const { window } = new JSDOM("<!doctype html><html><body></body></html>", { url: "https://boutique.test/" });
for (const key of ["document", "navigator", "MutationObserver", "Event", "KeyboardEvent", "MouseEvent", "Element", "HTMLElement", "Node", "AbortController", "AbortSignal"]) {
    Object.defineProperty(globalThis, key, { value: (window as unknown as Record<string, unknown>)[key], configurable: true, writable: true });
}
Object.defineProperty(globalThis, "window", { value: window, configurable: true, writable: true });

const { context, defineSetups, inject, provide } = await import("../src/setups.ts");

type Registry = Parameters<typeof defineSetups>[0];
type Calls = unknown[][];

const registry: Registry = {};
const warnings: unknown[][] = [];
console.warn = (...args: unknown[]) => void warnings.push(args);

const tick = () => new Promise((resolve) => setTimeout(resolve, 0));

async function mount(html: string): Promise<HTMLElement> {
    const host = document.createElement("div");
    host.innerHTML = html;
    document.body.append(host);
    await tick();
    await tick();
    return host;
}

type Options = { lazy?: boolean; refs?: string[]; many?: string[]; handlers?: string[] };

function add(name: string, setup: (element: Element, refs: any, context: { signal: AbortSignal }) => unknown, options: Options = {}) {
    const lists = { tags: [], refs: options.refs ?? [], many: options.many ?? [], handlers: options.handlers ?? [] };
    registry[name] = options.lazy ? { ...lists, load: async () => setup } : { ...lists, setup };
}

function key(target: EventTarget, init: Record<string, unknown>) {
    const event = new window.KeyboardEvent("keydown", { bubbles: true, cancelable: true, ...init });
    target.dispatchEvent(event as unknown as Event);
    return event;
}

function click(target: EventTarget, init: Record<string, unknown> = {}) {
    const event = new window.MouseEvent("click", { bubbles: true, cancelable: true, ...init });
    target.dispatchEvent(event as unknown as Event);
    return event;
}

before(() => defineSetups(registry));

afterEach(() => {
    document.body.innerHTML = "";
    warnings.length = 0;
});

test("handlers bind on the setup element and inside it", async () => {
    const calls: Calls = [];
    add("basic", () => ({
        own: (event: Event, element: Element) => calls.push(["own", event.type, element.id]),
        child: (event: Event, element: Element) => calls.push(["child", event.type, element.id]),
    }), { handlers: ["own", "child"] });
    const host = await mount(`<section id="root" bq-setup="basic" bq-on:click="own"><button id="b" bq-on:click="child"></button></section>`);

    click(host.querySelector("#b")!);
    assert.deepStrictEqual(calls, [
        ["child", "click", "b"],
        ["own", "click", "root"],
    ]);
});

test("kebab-case handler names call camelCase handlers", async () => {
    const calls: string[] = [];
    add("kebab", () => ({ filesPicked: () => calls.push("plain"), markDirty: () => calls.push("named") }), { handlers: ["filesPicked", "markDirty"] });
    const host = await mount(`<div bq-setup="kebab"><input bq-on:change="files-picked" bq-on:input="kebab.mark-dirty"></div>`);
    const input = host.querySelector("input")!;

    input.dispatchEvent(new window.Event("change") as unknown as Event);
    input.dispatchEvent(new window.Event("input") as unknown as Event);
    assert.deepStrictEqual(calls, ["plain", "named"]);
});

test("one attribute can call several handlers", async () => {
    const calls: string[] = [];
    add("outer", () => ({ schedule: () => calls.push("outer.schedule") }), { handlers: ["schedule"] });
    add("inner", () => ({ update: () => calls.push("inner.update"), count: () => calls.push("inner.count") }), { handlers: ["update", "count"] });
    const host = await mount(`<div bq-setup="outer"><form bq-setup="inner" bq-on:input=" update  outer.schedule count "><input></form></div>`);

    host.querySelector("input")!.dispatchEvent(new window.Event("input", { bubbles: true }) as unknown as Event);
    assert.deepStrictEqual(calls.filter((call) => call.startsWith("inner")), ["inner.update", "inner.count"]);
    assert.deepStrictEqual([...calls].sort(), ["inner.count", "inner.update", "outer.schedule"]);
    assert.deepStrictEqual(warnings, []);
});

test("inject waits for a provider that loads later", async () => {
    const theme = context<{ color: string }>("theme");
    const seen: string[] = [];
    add("late-provider", (element) => provide(element, theme, { color: "red" }), { lazy: true });
    add("early-child", async (element) => {
        seen.push((await inject(element, theme)).color);
    });
    await mount(`<div bq-setup="late-provider"><p><span bq-setup="early-child"></span></p></div>`);
    await tick();
    assert.deepStrictEqual(seen, ["red"]);
});

test("inject takes the nearest provider", async () => {
    const level = context<string>("level");
    const seen: string[] = [];
    add("outer-level", (element) => provide(element, level, "outer"));
    add("inner-level", (element) => provide(element, level, "inner"), { lazy: true });
    add("reader", async (element) => {
        seen.push(await inject(element, level));
    });
    await mount(`<div bq-setup="outer-level"><section bq-setup="inner-level"><b bq-setup="reader"></b></section><i bq-setup="reader"></i></div>`);
    await tick();
    assert.deepStrictEqual(seen.sort(), ["inner", "outer"]);
});

test("inject skips ancestors that do not provide the key", async () => {
    const level = context<string>("skip");
    const seen: string[] = [];
    add("skip-provider", (element) => provide(element, level, "found"));
    add("skip-middle", () => {}, { lazy: true });
    add("skip-reader", async (element) => {
        seen.push(await inject(element, level));
    });
    await mount(`<div bq-setup="skip-provider"><div bq-setup="skip-middle"><b bq-setup="skip-reader"></b></div></div>`);
    await tick();
    assert.deepStrictEqual(seen, ["found"]);
});

test("inject fails when no setup above provides the key", async () => {
    const missing = context<string>("missing");
    const errors: string[] = [];
    add("no-provider", () => {});
    add("orphan", async (element) => {
        await inject(element, missing).catch((error: Error) => errors.push(error.message));
    });
    add("broken-provider", () => {
        throw new Error("broken");
    });
    const error = console.error;
    console.error = () => {};
    try {
        await mount(`<div bq-setup="no-provider"><b bq-setup="orphan"></b></div><div bq-setup="broken-provider"><b bq-setup="orphan"></b></div>`);
        await tick();
    } finally {
        console.error = error;
    }
    assert.deepStrictEqual(errors, [
        `[bq] no setup above this element provides "missing"`,
        `[bq] no setup above this element provides "missing"`,
    ]);
});

test("refs go to the nearest setup that uses the name", async () => {
    const seen: Record<string, unknown> = {};
    add("tool-bar", (_element, refs) => void (seen.toolbar = refs), { refs: ["blockButton", "blockOption", "far"], many: ["blockOption"] });
    add("pop-menu", (_element, refs, { signal }) => void (seen.menu = { ...refs, signal: signal instanceof AbortSignal }), { refs: ["button", "panel"] });
    await mount(
        `<div bq-setup="tool-bar"><div bq-setup="pop-menu" bq-ref="far"><button id="b" bq-ref="button block-button"></button><div id="p" bq-ref="panel"><i id="o1" bq-ref="block-option"></i><i id="o2" bq-ref="block-option"></i></div></div></div>`,
    );
    const ids = (value: unknown) => (Array.isArray(value) ? value.map((element) => element.id) : (value as Element | undefined)?.id ?? value);
    const toolbar = seen.toolbar as Record<string, unknown>;
    const menu = seen.menu as Record<string, unknown>;
    assert.deepStrictEqual(Object.fromEntries(Object.entries(toolbar).map(([key, value]) => [key, ids(value)])), {
        blockOption: ["o1", "o2"],
        far: "",
        blockButton: "b",
    });
    assert.deepStrictEqual(Object.fromEntries(Object.entries(menu).map(([key, value]) => [key, ids(value)])), { button: "b", panel: "p", signal: true });
    assert.deepStrictEqual(warnings, []);
});

test("a prefix sends a ref to a named setup", async () => {
    const seen: Record<string, unknown> = {};
    add("outer-named", (_element, refs) => void (seen.outer = Object.keys(refs)), { refs: [] });
    add("inner-named", (_element, refs) => void (seen.inner = Object.keys(refs)), { refs: ["icon"] });
    await mount(`<div bq-setup="outer-named"><div bq-setup="inner-named"><i bq-ref="outer-named.icon"></i></div></div>`);
    assert.deepStrictEqual(seen, { outer: ["icon"], inner: [] });
});

test("handlers go to the nearest setup that uses the name", async () => {
    const calls: string[] = [];
    add("bar", () => ({ setBlock: () => calls.push("bar.setBlock") }), { handlers: ["setBlock"] });
    add("pop", () => ({ choose: () => calls.push("pop.choose") }), { handlers: ["choose"] });
    const host = await mount(`<div bq-setup="bar"><div bq-setup="pop"><button bq-on:click="set-block choose"></button></div></div>`);

    click(host.querySelector("button")!);
    assert.deepStrictEqual(calls.sort(), ["bar.setBlock", "pop.choose"]);
    assert.deepStrictEqual(warnings, []);
});

test("prevent and stop work without a handler", async () => {
    const calls: string[] = [];
    add("bare", () => ({ outer: () => calls.push("outer") }), { handlers: ["outer"] });
    const host = await mount(`<div bq-setup="bare" bq-on:click="outer"><a bq-on:mousedown|prevent></a><b bq-on:click|stop></b></div>`);

    const down = new window.MouseEvent("mousedown", { bubbles: true, cancelable: true });
    host.querySelector("a")!.dispatchEvent(down as unknown as Event);
    click(host.querySelector("b")!);
    click(host.querySelector("a")!);
    assert.strictEqual(down.defaultPrevented, true);
    assert.deepStrictEqual(calls, ["outer"]);
    assert.deepStrictEqual(warnings, []);
});

test("setups that return nothing still run", async () => {
    let ran = false;
    add("plain", () => {
        ran = true;
    });
    await mount(`<div bq-setup="plain"></div>`);
    assert.strictEqual(ran, true);
    assert.deepStrictEqual(warnings, []);
});

test("prevent and stop run only when the filters pass", async () => {
    const calls: string[] = [];
    add("keys", () => ({ close: () => calls.push("close") }), { handlers: ["close"] });
    const host = await mount(`<div bq-setup="keys"><input bq-on:keydown|escape|prevent|stop="close"></div>`);
    const input = host.querySelector("input")!;

    const other = key(input, { key: "a" });
    const escape = key(input, { key: "Escape" });
    assert.deepStrictEqual(calls, ["close"]);
    assert.strictEqual(other.defaultPrevented, false);
    assert.strictEqual(escape.defaultPrevented, true);
});

test("key names, letters and arrows", async () => {
    const calls: string[] = [];
    add("letters", () => ({ hit: (event: KeyboardEvent) => calls.push(event.key) }), { handlers: ["hit"] });
    const host = await mount(`<div bq-setup="letters"><input bq-on:keydown|s|enter|left="hit"></div>`);
    const input = host.querySelector("input")!;

    for (const name of ["s", "S", "Enter", "ArrowLeft", "ArrowRight", "x"]) key(input, { key: name });
    assert.deepStrictEqual(calls, ["s", "S", "Enter", "ArrowLeft"]);
});

test("system keys, mod and exact", async () => {
    const calls: string[] = [];
    Object.defineProperty(window.navigator, "platform", { value: "MacIntel", configurable: true });
    add("system", () => ({
        save: () => calls.push("save"),
        shifted: () => calls.push("shifted"),
    }), { handlers: ["save", "shifted"] });
    const host = await mount(`<div bq-setup="system"><input bq-on:keydown|mod|exact|s="save"><textarea bq-on:keydown|shift|enter="shifted"></textarea></div>`);
    const input = host.querySelector("input")!;
    const textarea = host.querySelector("textarea")!;

    key(input, { key: "s", ctrlKey: true });
    key(input, { key: "s", metaKey: true, shiftKey: true });
    key(input, { key: "s", metaKey: true });
    key(textarea, { key: "Enter" });
    key(textarea, { key: "Enter", shiftKey: true, ctrlKey: true });
    assert.deepStrictEqual(calls, ["save", "shifted"]);
});

test("mouse buttons", async () => {
    const calls: number[] = [];
    add("mouse", () => ({ middle: (event: MouseEvent) => calls.push(event.button) }), { handlers: ["middle"] });
    const host = await mount(`<div bq-setup="mouse"><a bq-on:mousedown|middle="middle"></a></div>`);
    const link = host.querySelector("a")!;

    for (const button of [0, 1, 2]) {
        link.dispatchEvent(new window.MouseEvent("mousedown", { bubbles: true, button }) as unknown as Event);
    }
    assert.deepStrictEqual(calls, [1]);
});

test("self ignores events from children", async () => {
    const calls: string[] = [];
    add("self", () => ({ backdrop: () => calls.push("backdrop") }), { handlers: ["backdrop"] });
    const host = await mount(`<div bq-setup="self" bq-on:click|self="backdrop"><p></p></div>`);

    click(host.querySelector("p")!);
    click(host.querySelector("[bq-setup]")!);
    assert.deepStrictEqual(calls, ["backdrop"]);
});

test("once runs a single time, after the filters pass", async () => {
    const calls: string[] = [];
    add("once", () => ({ first: () => calls.push("first") }), { handlers: ["first"] });
    const host = await mount(`<div bq-setup="once"><input bq-on:keydown|enter|once="first"></div>`);
    const input = host.querySelector("input")!;

    key(input, { key: "a" });
    key(input, { key: "Enter" });
    key(input, { key: "Enter" });
    assert.deepStrictEqual(calls, ["first"]);
});

test("window and document targets", async () => {
    const calls: string[] = [];
    add("targets", () => ({
        outside: (_event: Event, element: Element) => calls.push(`document:${element.localName}`),
        resize: () => calls.push("window"),
    }), { handlers: ["outside", "resize"] });
    await mount(`<details bq-setup="targets" bq-on:click|document="outside" bq-on:resize|window="resize"></details>`);

    click(document.body);
    window.dispatchEvent(new window.Event("resize") as unknown as Event);
    assert.deepStrictEqual(calls, ["document:details", "window"]);
});

test("listeners stop when the setup element is removed", async () => {
    const calls: string[] = [];
    add("removed", () => ({ outside: () => calls.push("outside") }), { handlers: ["outside"] });
    const host = await mount(`<div bq-setup="removed" bq-on:click|document="outside"></div>`);

    click(document.body);
    host.remove();
    await tick();
    click(document.body);
    assert.deepStrictEqual(calls, ["outside"]);
});

test("listeners stop when bq-setup changes", async () => {
    const calls: string[] = [];
    add("changing", () => ({ hit: () => calls.push("hit") }), { handlers: ["hit"] });
    const host = await mount(`<button bq-setup="changing" bq-on:click="hit"></button>`);
    const button = host.querySelector("button")!;

    click(button);
    button.removeAttribute("bq-setup");
    await tick();
    click(button);
    assert.deepStrictEqual(calls, ["hit"]);
});

test("two setups that use the same name need a setup name", async () => {
    const calls: string[] = [];
    add("first-setup", () => ({ save: () => calls.push("first") }), { handlers: ["save"] });
    add("second-setup", () => ({ save: () => calls.push("second") }), { handlers: ["save"] });
    const host = await mount(
        `<form bq-setup="first-setup second-setup"><button id="named" bq-on:click="second-setup.save"></button><button id="bare" bq-on:click="save"></button></form>`,
    );

    click(host.querySelector("#named")!);
    click(host.querySelector("#bare")!);
    assert.deepStrictEqual(calls, ["second"]);
    assert.ok(String(warnings[0]?.[0]).includes("is used by more than one setup"));
});

test("two setups on one element, each with its own name", async () => {
    const calls: string[] = [];
    add("form-a", () => ({ update: () => calls.push("update") }), { handlers: ["update"] });
    add("form-b", () => ({ schedule: () => calls.push("schedule") }), { handlers: ["schedule"] });
    const host = await mount(`<form bq-setup="form-a form-b" bq-on:input="update schedule"><input></form>`);

    host.querySelector("input")!.dispatchEvent(new window.Event("input", { bubbles: true }) as unknown as Event);
    assert.deepStrictEqual(calls.sort(), ["schedule", "update"]);
    assert.deepStrictEqual(warnings, []);
});

test("nested setups: nearest by default, outer by name", async () => {
    const calls: string[] = [];
    add("outer-form", () => ({ changed: () => calls.push("outer") }), { handlers: ["changed"] });
    add("inner-field", () => ({ changed: () => calls.push("inner") }), { handlers: ["changed"] });
    const host = await mount(
        `<form bq-setup="outer-form"><div bq-setup="inner-field"><input id="near" bq-on:input="changed"><input id="far" bq-on:input="outer-form.changed"></div></form>`,
    );

    host.querySelector("#near")!.dispatchEvent(new window.Event("input") as unknown as Event);
    host.querySelector("#far")!.dispatchEvent(new window.Event("input") as unknown as Event);
    assert.deepStrictEqual(calls, ["inner", "outer"]);
});

test("elements added later bind once, also after a move", async () => {
    const calls: string[] = [];
    add("later", () => ({ hit: () => calls.push("hit") }), { handlers: ["hit"] });
    const host = await mount(`<ul bq-setup="later"></ul>`);
    const list = host.querySelector("ul")!;

    list.insertAdjacentHTML("beforeend", `<li><button bq-on:click="hit"></button></li>`);
    await tick();
    const item = list.querySelector("li")!;
    list.append(item);
    await tick();

    click(list.querySelector("button")!);
    assert.deepStrictEqual(calls, ["hit"]);
});

test("async and lazy setups bind after they resolve", async () => {
    const calls: string[] = [];
    add("async", async () => ({ hit: () => calls.push("async") }), { handlers: ["hit"] });
    add("lazy", () => ({ hit: () => calls.push("lazy") }), { lazy: true, handlers: ["hit"] });
    const host = await mount(`<button id="a" bq-setup="async" bq-on:click="hit"></button><button id="l" bq-setup="lazy" bq-on:click="hit"></button>`);
    await tick();

    click(host.querySelector("#a")!);
    click(host.querySelector("#l")!);
    assert.deepStrictEqual(calls, ["async", "lazy"]);
});

test("disconnect runs when the element is removed", async () => {
    const calls: string[] = [];
    add("stops", () => ({ disconnect: () => calls.push("disconnect") }));
    const host = await mount(`<div bq-setup="stops"></div>`);

    assert.deepStrictEqual(calls, []);
    host.remove();
    await tick();
    assert.deepStrictEqual(calls, ["disconnect"]);
});

test("disconnect runs when bq-setup changes, with handlers", async () => {
    const calls: string[] = [];
    add("stops-with-handlers", () => ({
        hit: () => calls.push("hit"),
        disconnect: () => calls.push("disconnect"),
    }), { handlers: ["hit"] });
    const host = await mount(`<button bq-setup="stops-with-handlers" bq-on:click="hit"></button>`);
    const button = host.querySelector("button")!;

    click(button);
    button.setAttribute("bq-setup", "");
    await tick();
    click(button);
    assert.deepStrictEqual(calls, ["hit", "disconnect"]);
});

test("disconnect runs at once when an async setup resolves after it stopped", async () => {
    const calls: string[] = [];
    let resolve: (value: unknown) => void = () => {};
    add("slow", () => new Promise((done) => (resolve = done)));
    const host = await mount(`<div bq-setup="slow"></div>`);

    host.remove();
    await tick();
    resolve({ disconnect: () => calls.push("disconnect") });
    await tick();
    assert.deepStrictEqual(calls, ["disconnect"]);
});

test("disconnect is not a handler and its errors are caught", async () => {
    const errors: unknown[][] = [];
    const original = console.error;
    console.error = (...args: unknown[]) => void errors.push(args);
    let stopped = 0;
    add("bad-disconnect", () => ({
        disconnect: () => {
            stopped += 1;
            throw new Error("stop");
        },
    }));
    const host = await mount(`<button bq-setup="bad-disconnect" bq-on:click="disconnect"></button>`);

    click(host.querySelector("button")!);
    assert.strictEqual(stopped, 0);
    assert.ok(String(warnings[0]?.[0]).includes(`no setup above uses bq-on:click="disconnect"`));
    host.remove();
    await tick();
    console.error = original;
    assert.strictEqual(stopped, 1);
    assert.ok(String(errors[0]?.[0]).includes(`setup "bad-disconnect" failed`));
});

test("a missing handler warns", async () => {
    add("missing", () => ({}), { handlers: ["nothing"] });
    await mount(`<button bq-setup="missing" bq-on:click="nothing"></button>`);
    assert.ok(String(warnings[0]?.[0]).includes(`setup "missing" has no handler "nothing"`));
});

test("handler errors are caught", async () => {
    const errors: unknown[][] = [];
    const original = console.error;
    console.error = (...args: unknown[]) => void errors.push(args);
    add("throws", () => ({
        boom: () => {
            throw new Error("boom");
        },
    }), { handlers: ["boom"] });
    const host = await mount(`<button bq-setup="throws" bq-on:click="boom"></button>`);

    click(host.querySelector("button")!);
    console.error = original;
    assert.ok(String(errors[0]?.[0]).includes(`handler "throws.boom" failed`));
});
