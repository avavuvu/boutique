import "../bindings/boutique/style/index.css"
import "../bindings/boutique/style/layout.css"
import "../bindings/boutique/style/components.css"

import { defineSetups } from "../bindings/boutique/setups"
import { registry } from "../bindings/registry"

import.meta.glob("./**/*.css", { eager: true })

defineSetups(registry)
