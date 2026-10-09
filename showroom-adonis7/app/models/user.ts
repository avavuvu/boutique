import { DateTime } from 'luxon'
import { nanoid } from 'nanoid'
import hash from '@adonisjs/core/services/hash'
import { BaseModel, column, beforeCreate } from '@adonisjs/lucid/orm'
export default class User extends BaseModel {
    @column({ isPrimary: true })
    declare id: string

    @beforeCreate()
    static assignId(user: User) {
        user.id = nanoid()
    }

    @column()
    declare username: string

    @column()
    declare email: string

    @column({ serializeAs: null })
    declare password: string | null

    @column.dateTime({ autoCreate: true })
    declare createdAt: DateTime

    @column.dateTime({ autoCreate: true, autoUpdate: true })
    declare updatedAt: DateTime | null

    static async verifyCredentials(email: string, password: string): Promise<User> {
        const user = await User.query().where('email', email).orWhere('username', email).first()

        if (!user || !user.password) {
            throw new Error('E_INVALID_CREDENTIALS')
        }

        const isValid = await hash.verify(user.password, password)
        if (!isValid) {
            throw new Error('E_INVALID_CREDENTIALS')
        }

        return user
    }

    @beforeCreate()
    static async hashPassword(user: User) {
        if (user.password) {
            user.password = await hash.make(user.password)
        }
    }
}
