/*
|--------------------------------------------------------------------------
| Routes file
|--------------------------------------------------------------------------
|
| The routes file is used for defining the HTTP routes.
|
*/

import env from '#start/env'
import { middleware } from '#start/kernel'
import { controllers } from '#generated/controllers'
import router from '@adonisjs/core/services/router'

const domain = env.get('HOST')

router.on('/').render('pages/home').as('home')
router
    .group(() => {
        router.get('signup', [controllers.NewAccount, 'create'])
        router.post('signup', [controllers.NewAccount, 'store'])
        router.get('login', [controllers.Session, 'create'])
        router.post('login', [controllers.Session, 'store'])
    })
    .use(middleware.guest())

// app.showroom.you
router
    .group(() => {
        // Authenticated
        router
            .group(() => {
                router.get('/', [controllers.Dashboard, 'index'])
                router.post('logout', [controllers.Session, 'destroy'])
                router.get('edit/:id', [controllers.Editor, 'show'])
                router.get('edit/:id/newsletter', [controllers.Editor, 'get'])
                router.patch('edit/:id', [controllers.Editor, 'update'])
                router.get('edit/:id/export', [controllers.Editor, 'export'])
                router.post('edit/:id/import', [controllers.Editor, 'import'])
            })
            .use(middleware.auth())
    })
    .domain(`app.${domain}`)

// {username}.showroom.you
router
    .group(() => {
        router.get('/', [controllers.Profile, 'show'])
        router.get(':slug', [controllers.Profile, 'newsletter'])
    })
    .domain(`:username.${domain}`)
