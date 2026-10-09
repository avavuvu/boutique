import '@adonisjs/core/types/http'

type ParamValue = string | number | bigint | boolean

export type ScannedRoutes = {
  ALL: {
    'home': { paramsTuple?: []; params?: {} }
    'new_account.create': { paramsTuple?: []; params?: {} }
    'new_account.store': { paramsTuple?: []; params?: {} }
    'session.create': { paramsTuple?: []; params?: {} }
    'session.store': { paramsTuple?: []; params?: {} }
    'dashboard.index': { paramsTuple?: []; params?: {} }
    'session.destroy': { paramsTuple?: []; params?: {} }
    'editor.show': { paramsTuple: [ParamValue]; params: {'id': ParamValue} }
    'editor.get': { paramsTuple: [ParamValue]; params: {'id': ParamValue} }
    'editor.update': { paramsTuple: [ParamValue]; params: {'id': ParamValue} }
    'editor.export': { paramsTuple: [ParamValue]; params: {'id': ParamValue} }
    'editor.import': { paramsTuple: [ParamValue]; params: {'id': ParamValue} }
    'profile.show': { paramsTuple?: []; params?: {} }
    'profile.newsletter': { paramsTuple: [ParamValue]; params: {'slug': ParamValue} }
  }
  GET: {
    'home': { paramsTuple?: []; params?: {} }
    'new_account.create': { paramsTuple?: []; params?: {} }
    'session.create': { paramsTuple?: []; params?: {} }
    'dashboard.index': { paramsTuple?: []; params?: {} }
    'editor.show': { paramsTuple: [ParamValue]; params: {'id': ParamValue} }
    'editor.get': { paramsTuple: [ParamValue]; params: {'id': ParamValue} }
    'editor.export': { paramsTuple: [ParamValue]; params: {'id': ParamValue} }
    'profile.show': { paramsTuple?: []; params?: {} }
    'profile.newsletter': { paramsTuple: [ParamValue]; params: {'slug': ParamValue} }
  }
  HEAD: {
    'home': { paramsTuple?: []; params?: {} }
    'new_account.create': { paramsTuple?: []; params?: {} }
    'session.create': { paramsTuple?: []; params?: {} }
    'dashboard.index': { paramsTuple?: []; params?: {} }
    'editor.show': { paramsTuple: [ParamValue]; params: {'id': ParamValue} }
    'editor.get': { paramsTuple: [ParamValue]; params: {'id': ParamValue} }
    'editor.export': { paramsTuple: [ParamValue]; params: {'id': ParamValue} }
    'profile.show': { paramsTuple?: []; params?: {} }
    'profile.newsletter': { paramsTuple: [ParamValue]; params: {'slug': ParamValue} }
  }
  POST: {
    'new_account.store': { paramsTuple?: []; params?: {} }
    'session.store': { paramsTuple?: []; params?: {} }
    'session.destroy': { paramsTuple?: []; params?: {} }
    'editor.import': { paramsTuple: [ParamValue]; params: {'id': ParamValue} }
  }
  PATCH: {
    'editor.update': { paramsTuple: [ParamValue]; params: {'id': ParamValue} }
  }
}
declare module '@adonisjs/core/types/http' {
  export interface RoutesList extends ScannedRoutes {}
}