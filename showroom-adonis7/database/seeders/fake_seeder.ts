import { NewsletterFactory } from '#database/factories/newsletter_factory'
import { UserFactory } from '#database/factories/user_factory'
import { BaseSeeder } from '@adonisjs/lucid/seeders'

export default class extends BaseSeeder {
    async run() {
        const users = await UserFactory.createMany(3)
        for (const user of users) {
            await NewsletterFactory.merge({ userId: user.id }).createMany(5)
        }
    }
}
