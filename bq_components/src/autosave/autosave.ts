const FORM = "form[data-autosave]";
const DIRTY = "data-dirty";
const PENDING = "htmx-request";
const UNSAVED = `${FORM}[${DIRTY}], ${FORM}.${PENDING}`;

function formOf(target: EventTarget | null): HTMLFormElement | null {
    return target instanceof Element ? target.closest<HTMLFormElement>(FORM) : null;
}

function unsaved(): HTMLFormElement[] {
    return Array.from(document.querySelectorAll<HTMLFormElement>(UNSAVED));
}

function flushAll(): Promise<void> {
    const forms = unsaved();
    if (forms.length === 0) return Promise.resolve();

    return new Promise((resolve) => {
        const done = () => {
            document.removeEventListener("htmx:finally:request", check);
            clearTimeout(deadline);
            resolve();
        };
        const check = () => {
            if (unsaved().length === 0) done();
        };
        document.addEventListener("htmx:finally:request", check);
        const deadline = setTimeout(done, 3000);

        for (const form of forms) {
            if (form.hasAttribute(DIRTY) && !form.classList.contains(PENDING)) form.requestSubmit();
        }
    });
}

function attachAutosave(): void {
    document.addEventListener("input", (event) => {
        formOf(event.target)?.setAttribute(DIRTY, "");
    });

    document.addEventListener("htmx:before:request", (event) => {
        formOf(event.target)?.removeAttribute(DIRTY);
    });

    document.addEventListener("htmx:response:error", (event) => {
        formOf(event.target)?.setAttribute(DIRTY, "");
    });

    document.addEventListener("click", async (event) => {
        const link = (event.target as Element).closest<HTMLAnchorElement>("a[data-leave]");
        if (!link || unsaved().length === 0) return;

        event.preventDefault();
        link.setAttribute("aria-busy", "true");
        await flushAll();
        window.location.href = link.href;
    });

    window.addEventListener("beforeunload", (event) => {
        if (unsaved().length > 0) event.preventDefault();
    });
}

attachAutosave();
