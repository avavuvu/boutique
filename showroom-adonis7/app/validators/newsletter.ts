import vine from '@vinejs/vine'

/**
 * `content`: ProseMirror JSON document,
 */
export const updateNewsletterValidator = vine.create({
    title: vine.string().trim().minLength(1).optional(),
    slug: vine.string().trim().optional(),
    subtitle: vine.string().trim().nullable().optional(),
    content: vine.string(),
})
