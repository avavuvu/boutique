import type { Setup } from "../setups";

type PasswordToggle = Setup<"div", { toggle: "button" }>;

export const passwordToggle: PasswordToggle = (shell, { toggle }) => {
    const input = shell.querySelector("input");
    if (!input) return;

    toggle.addEventListener("click", () => {
        const hidden = input.type === "password";
        input.type = hidden ? "text" : "password";
        toggle.textContent = hidden ? "Hide" : "Show";
    });
};
