import { fileURLToPath } from "node:url"
import js from "@eslint/js"
import stylistic from "@stylistic/eslint-plugin"
import { defineConfig, globalIgnores } from "eslint/config"
import tseslint from "typescript-eslint"

const booleanPrefixes = ["is", "has", "should", "can", "was", "did"]

const abbreviations = [
    "arr",
    "btn",
    "cb",
    "ctx",
    "e",
    "el",
    "elem",
    "err",
    "ev",
    "evt",
    "idx",
    "msg",
    "obj",
    "req",
    "res",
    "str",
    "tmp",
    "tr",
    "txn",
    "val",
]

const root = fileURLToPath(new URL("../../..", import.meta.url))

const arrowFunctionMessage = "Use an arrow function unless it needs its own `this`, is a generator, or is a class method."

export default defineConfig(
    globalIgnores(["bindings/**", "public/build/**", "node_modules/**", "target/**"]),
    js.configs.recommended,
    tseslint.configs.recommended,
    stylistic.configs.customize({
        indent: 4,
        quotes: "double",
        semi: false,
        braceStyle: "1tbs",
        arrowParens: true,
        commaDangle: "always-multiline",
        quoteProps: "as-needed",
        jsx: false,
    }),
    {
        rules: {
            curly: ["error", "all"],
            "func-style": ["error", "expression"],
            "prefer-arrow-callback": "error",
            "no-restricted-syntax": [
                "error",
                {
                    selector: ":not(MethodDefinition, Property) > FunctionExpression[generator=false]:not(:has(ThisExpression))",
                    message: arrowFunctionMessage,
                },
                {
                    selector: "Property[method=true] > FunctionExpression[generator=false]:not(:has(ThisExpression))",
                    message: arrowFunctionMessage,
                },
            ],
            "id-denylist": ["error", ...abbreviations],
            "no-empty": ["error", { allowEmptyCatch: true }],
            "@stylistic/semi": ["error", "never", { beforeStatementContinuationChars: "always" }],
            "@stylistic/brace-style": ["error", "1tbs", { allowSingleLine: false }],
            "@stylistic/quotes": ["error", "double", { avoidEscape: true, allowTemplateLiterals: "always" }],
            "@stylistic/operator-linebreak": ["error", "after", { overrides: { "?": "before", ":": "before", "|": "before", "&": "before" } }],
        },
    },
    {
        files: ["**/*.ts"],
        languageOptions: {
            parserOptions: {
                projectService: { allowDefaultProject: ["vite.config.ts"] },
                tsconfigRootDir: root,
            },
        },
        rules: {
            "no-nested-ternary": ["error"],
            "@typescript-eslint/array-type": ["error", { default: "generic", readonly: "generic" }],
            "@typescript-eslint/naming-convention": [
                "error",
                {
                    selector: ["variable", "parameter", "parameterProperty", "classProperty", "typeProperty", "accessor"],
                    types: ["boolean"],
                    format: ["PascalCase"],
                    prefix: booleanPrefixes,
                    leadingUnderscore: "allow",
                },
                {
                    selector: ["variable", "parameter"],
                    modifiers: ["destructured"],
                    types: ["boolean"],
                    format: null,
                },
            ],
        },
    },
)
