import { DateTime } from 'luxon'
import { BaseModel, beforeCreate, beforeSave, belongsTo, column } from '@adonisjs/lucid/orm'
import type { BelongsTo } from '@adonisjs/lucid/types/relations'
import string from '@adonisjs/core/helpers/string'
import { nanoid } from 'nanoid'
import User from '#models/user'

export default class Newsletter extends BaseModel {
    static selfAssignPrimaryKey = true

    @column({ isPrimary: true })
    declare id: string

    @beforeCreate()
    static assignId(newsletter: Newsletter) {
        newsletter.id = nanoid(14)
    }

    @column()
    declare userId: string

    @belongsTo(() => User)
    declare user: BelongsTo<typeof User>

    @column()
    declare title: string

    @column()
    declare slug: string

    @beforeSave()
    static slugify(newsletter: Newsletter) {
        if (newsletter.$dirty.title && !newsletter.$dirty.slug) {
            newsletter.slug = string.slug(newsletter.title, { lower: true })
        }
    }

    @column()
    declare subtitle: string | null

    @column()
    declare content: string

    @column()
    declare sent: boolean

    @column.dateTime({ autoCreate: true })
    declare createdAt: DateTime

    @column.dateTime({ autoCreate: true, autoUpdate: true })
    declare updatedAt: DateTime
}
