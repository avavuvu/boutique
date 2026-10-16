import type { Setup } from "../setups";

type PasswordToggle = Setup<"div", {}, { toggle: { event: "click"; tag: "button" } }>;

export const passwordToggle: PasswordToggle = (shell) => ({
    toggle(_event, button) {
        const input = shell.querySelector("input");
        if (!input) return;

        const hidden = input.type === "password";
        input.type = hidden ? "text" : "password";
        button.setAttribute("aria-pressed", String(hidden));
    },
});
