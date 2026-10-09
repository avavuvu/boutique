import factory from '@adonisjs/lucid/factories'
import User from '#models/user'

export const UserFactory = factory
    .define(User, async ({ faker }) => {
        return {
            username: faker.internet.username().replace('_', '-').replace('.', '').toLowerCase(),
            email: faker.internet.email(),
            password: faker.internet.password(),
        }
    })
    .build()
