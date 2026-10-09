import { BaseSchema } from '@adonisjs/lucid/schema'

export default class extends BaseSchema {
    protected tableName = 'newsletters'

    async up() {
        this.schema.createTable(this.tableName, (table) => {
            table.string('id').primary()
            table.string('user_id').notNullable().references('id').inTable('users')

            table.string('title').notNullable().defaultTo('Untitled')
            table.string('slug').notNullable()
            table.string('subtitle').nullable()

            table.text('content').notNullable().defaultTo('{"type":"doc","content":[]}')

            table.boolean('sent').notNullable().defaultTo(false)

            table.timestamp('created_at').notNullable()
            table.timestamp('updated_at').nullable()
        })
    }

    async down() {
        this.schema.dropTable(this.tableName)
    }
}
