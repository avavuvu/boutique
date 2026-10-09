import factory from '@adonisjs/lucid/factories'
import { UserFactory } from './user_factory.js'
import Newsletter from '#models/newsletter'

export const NewsletterFactory = factory
    .define(Newsletter, async ({ faker }) => {
        return {
            title: `On ${faker.word.noun()} and ${faker.word.noun()}`,
            content: faker.lorem.paragraphs(4, '\n'),
        }
    })
    .relation('user', () => UserFactory)
    .build()
