import type { HttpContext } from '@adonisjs/core/http'
import { readFile } from 'node:fs/promises'
import Newsletter from '#models/newsletter'
import { updateNewsletterValidator } from '#validators/newsletter'
import MarkdownService from '#services/markdown_service'

export default class EditorController {
    async show({ params, view, auth }: HttpContext) {
        const user = auth.user!
        const newsletter = await this.findNewsletter(user.id, params.id)

        return view.render('pages/editor/index', { newsletter })
    }

    async get({ params, response, auth }: HttpContext) {
        const user = auth.user!
        const newsletter = await this.findNewsletter(user.id, params.id)

        return response.ok({ newsletter })
    }

    async update({ params, request, response, auth }: HttpContext) {
        const user = auth.user!
        const newsletter = await this.findNewsletter(user.id, params.id)

        const payload = await request.validateUsing(updateNewsletterValidator)

        newsletter.merge({
            title: payload.title ?? newsletter.title,
            slug: payload.slug ?? newsletter.slug,
            subtitle: payload.subtitle ?? null,
            content: payload.content,
        })

        await newsletter.save()

        if (request.accepts(['json'])) {
            return response.ok({ saved: true })
        }

        return response.redirect().back()
    }

    // removed til i create the view
    // async preview({ params, view, auth }: HttpContext) {
    //     const user = auth.user!
    //     const newsletter = await this.findNewsletter(user.id, params.id)

    //     return view.render('pages/editor/preview', { newsletter })
    // }

    async export({ params, response, auth }: HttpContext) {
        const user = auth.user!
        const newsletter = await this.findNewsletter(user.id, params.id)

        const markdown = MarkdownService.serialize(newsletter.content)
        const filename = `${newsletter.slug || newsletter.id}.md`

        response.header('Content-Disposition', `attachment; filename="${filename}"`)
        response.header('Content-Type', 'text/markdown; charset=utf-8')

        return response.send(markdown)
    }

    async import({ params, request, response, auth, session }: HttpContext) {
        const user = auth.user!
        const newsletter = await this.findNewsletter(user.id, params.id)

        const file = request.file('file', {
            size: '1mb',
            extnames: ['md', 'txt'],
        })

        if (!file || !file.isValid) {
            session.flash('errors', { file: 'Please upload a valid .md file (max 1 MB).' })
            return response.redirect().back()
        }

        const markdown = await readFile(file.tmpPath!, 'utf-8')
        const pmJson = MarkdownService.parse(markdown)

        newsletter.content = JSON.stringify(pmJson)
        await newsletter.save()

        return response.redirect().back()
    }

    private async findNewsletter(userId: string, id: string) {
        return Newsletter.query().where('id', id).where('user_id', userId).firstOrFail()
    }
}
