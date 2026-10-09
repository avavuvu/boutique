import vine from '@vinejs/vine'

/**
 * Shared rules for email and password.
 */
const email = () => vine.string().email().maxLength(254)
const password = () => vine.string().minLength(8).maxLength(32)

/**
 * Validator to use when performing self-signup
 */
export const signupValidator = vine.create({
    username: vine
        .string()
        .trim()
        .minLength(2)
        .maxLength(50)
        .regex(/^[a-z0-9_-]+$/)
        .unique({ table: 'users', column: 'username' }),
    email: email().unique({ table: 'users', column: 'email' }),
    password: password(),
})
