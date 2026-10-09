import Newsletter from '#models/newsletter'
import type User from '#models/user'
import { type HttpContext } from '@adonisjs/core/http'
import vine from '@vinejs/vine'

export default class NewslettersController {
    async index({ auth, params, view }: HttpContext) {
        const user = auth.user! // why can i guarantee this not undefined?
        const newsletter = await this.findNewsletter(user, params.id)

        return view.render('pages/edit/index', {
            newsletter,
        })
    }

    async put({ auth, params, request, response }: HttpContext) {
        const user = auth.user!
        const newsletter = await this.findNewsletter(user, params.id)

        const validator = vine.create(
            vine.object({
                title: vine.string().trim().optional(),
                slug: vine.string().trim().optional(),
                subtitle: vine.string().trim().optional().nullable(),
                content: vine.string(),
            })
        )

        const payload = await request.validateUsing(validator)

        newsletter.merge({
            title: payload.title,
            slug: payload.slug,
            subtitle: payload.subtitle,
            content: payload.content,
        })

        await newsletter.save()

        return response.redirect().back()
    }

    private async findNewsletter(user: User, id: string) {
        const newsletter = await Newsletter.query()
            .where('id', id)
            .where('user_id', user.id)
            .firstOrFail()

        return newsletter
    }
}
