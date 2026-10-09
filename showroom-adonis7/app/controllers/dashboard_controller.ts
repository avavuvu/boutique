import Newsletter from '#models/newsletter'
import type { HttpContext } from '@adonisjs/core/http'

export default class DashboardController {
    async index({ auth, view }: HttpContext) {
        const user = auth.user!
        const newsletters = await Newsletter.query()
            .where('userId', user.id)
            .orderBy('createdAt', 'desc')

        return view.render('pages/dashboard/index', { user, newsletters })
    }
}
