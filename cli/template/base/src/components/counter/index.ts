import type { Counter } from "@setups"

export const counter: Counter = (root, { count }) => {
    let value = Number(root.dataset.start)

    const render = () => {
        count.value = String(value)
    }

    return {
        increment: () => {
            value += 1
            render()
        },
        decrement: () => {
            value -= 1
            render()
        },
    }
}
