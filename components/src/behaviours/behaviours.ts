const FEEDBACK_MS = 1500;

function flash(button: HTMLElement, text: string): void {
    const original = button.textContent;
    button.textContent = text;
    setTimeout(() => (button.textContent = original), FEEDBACK_MS);
}

document.addEventListener("click", (event) => {
    const target = event.target as Element;

    const copy = target.closest<HTMLElement>("[data-copy]");
    if (copy) {
        const url = copy.dataset.copy!;
        if (copy.hasAttribute("data-share") && navigator.share) {
            navigator.share({ url }).catch(() => {});
        } else {
            navigator.clipboard
                .writeText(url)
                .then(() => flash(copy, copy.dataset.copied ?? "Copied"))
                .catch(() => {});
        }
        return;
    }

    const select = target.closest<HTMLInputElement>("input[data-select-all]");
    if (select) select.select();

    const opener = target.closest<HTMLElement>("[data-open]");
    if (opener) {
        const details = document.getElementById(opener.dataset.open!);
        if (details instanceof HTMLDetailsElement) details.open = true;
    }
});

document.addEventListener("keydown", (event) => {
    if (event.key !== "Enter" || event.shiftKey) return;
    const field = (event.target as Element).closest<HTMLTextAreaElement>("textarea[data-submit-on-enter]");
    if (!field) return;
    event.preventDefault();
    field.form?.requestSubmit();
});
