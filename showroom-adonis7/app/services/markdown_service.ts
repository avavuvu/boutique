import { defaultMarkdownParser, defaultMarkdownSerializer } from 'prosemirror-markdown'
import type { Node } from 'prosemirror-model'

export default class MarkdownService {
    static serialize(pmJson: string | Node): string {
        const doc: Node = typeof pmJson === 'string' ? JSON.parse(pmJson) : pmJson

        return defaultMarkdownSerializer.serialize(doc)
    }

    static parse(markdown: string) {
        return defaultMarkdownParser.parse(markdown)
    }
}
