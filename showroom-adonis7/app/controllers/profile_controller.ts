import Newsletter from '#models/newsletter'
import User from '#models/user'
import type { HttpContext } from '@adonisjs/core/http'

export default class ProfileController {
    async show({ params, view }: HttpContext) {
        console.log(params)

        const user = await User.query().where('username', params.username).firstOrFail()
        const newsletters = await Newsletter.query()
            .where('userId', user.id)
            .orderBy('createdAt', 'desc')

        return view.render('pages/profile/index', { user, newsletters })
    }

    async newsletter({ params, view }: HttpContext) {
        const user = await User.query().where('username', params.username).firstOrFail()
        const newsletter = await Newsletter.query()
            .where('userId', user.id)
            .where('slug', params.slug)
            .firstOrFail()

        return view.render('pages/profile/newsletter', { user, newsletter })
    }
}
