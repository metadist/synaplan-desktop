import { describe, expect, it } from 'vitest'
import { turnNeedsFileTools } from '@/lib/turnRouting'

const cases: Array<[string, string, boolean]> = [
  ['en', 'Read notes.md and tell me the title', true],
  ['en', 'Write the sources into sources.md', true],
  ['en', 'Look in the project folder for the contract', true],
  ['en', 'What is the current LTS version of Node.js?', false],
  ['en', 'What does the attached file say?', false],
  ['de', 'Lies vertrag.pdf und nenne die Frist', true],
  ['de', 'Schreib die Quellen in quellen.md', true],
  ['de', 'Schau im Projektordner nach dem Vertrag', true],
  ['de', 'Was kostet ein Wartungsvertrag für Klimaanlagen in Bayern ungefähr?', false],
  ['de', 'Was steht in der angehängten Datei?', false],
  ['es', 'Lee contrato.pdf y dime la fecha', true],
  ['es', 'Escribe las fuentes en fuentes.md', true],
  ['es', 'Mira en la carpeta del proyecto', true],
  ['es', '¿Cuánto cuesta un contrato de mantenimiento?', false],
  ['es', '¿Qué dice el archivo adjunto?', false],
  ['fr', 'Lis contrat.pdf et dis-moi la date', true],
  ['fr', 'Écris les sources dans sources.md', true],
  ['fr', 'Regarde dans le dossier du projet', true],
  ['fr', 'Quel est le prix d’un contrat de maintenance ?', false],
  ['fr', 'Que dit le fichier joint ?', false],
  ['tr', 'sozlesme.pdf dosyasını oku', true],
  ['tr', 'Kaynakları kaynaklar.md içine yaz', true],
  ['tr', 'Proje klasörüne bak', true],
  ['tr', 'Bir bakım sözleşmesi yaklaşık ne kadar?', false],
  ['tr', 'Ekli dosyada ne yazıyor?', false],
]

describe('turnNeedsFileTools', () => {
  it.each(cases)('%s: %s → %s', (_locale, text, expected) => {
    expect(turnNeedsFileTools(text)).toBe(expected)
  })

  it('does not treat a plain question that mentions Node.js as a file', () => {
    expect(turnNeedsFileTools('Reply with the single word PONG')).toBe(false)
    expect(turnNeedsFileTools('What is Node.js used for?')).toBe(false)
  })

  it('does not treat a link to a document as a local file', () => {
    expect(turnNeedsFileTools('What does https://example.com/guide.pdf say?')).toBe(false)
    expect(turnNeedsFileTools('Summarise www.example.com/notes.md')).toBe(false)
  })
})
