/** Mirrors `synaplan-core::generation::classify` so Vue still routes a
 * create-request when the Rust command is not in the running binary yet. */

export type GenerationKind = 'image' | 'audio' | 'video' | 'document'

export function classifyGenerationLocal(text: string): GenerationKind | null {
  const trimmed = text.trim()
  if (!trimmed) {
    return null
  }
  const lower = trimmed.toLowerCase()

  if (looksLikeSlash(lower, 'pic') || looksLikeSlash(lower, 'img')) {
    return 'image'
  }
  if (looksLikeSlash(lower, 'tts') || looksLikeSlash(lower, 'speak')) {
    return 'audio'
  }
  if (looksLikeSlash(lower, 'vid')) {
    return 'video'
  }
  if (looksLikeSlash(lower, 'doc')) {
    return 'document'
  }

  if (isImageRequest(lower)) {
    return 'image'
  }
  if (isAudioRequest(lower)) {
    return 'audio'
  }
  if (isVideoRequest(lower)) {
    return 'video'
  }
  if (isDocumentRequest(lower)) {
    return 'document'
  }
  return null
}

function looksLikeSlash(lower: string, cmd: string): boolean {
  return lower === `/${cmd}` || lower.startsWith(`/${cmd} `) || lower.startsWith(`/${cmd}\n`)
}

function isImageRequest(lower: string): boolean {
  return (
    containsAny(lower, [
      'ein echtes bild',
      'ein bild',
      'echte bild',
      'generate an image',
      'create an image',
      'create a picture',
      'draw a',
      'draw me',
      'paint a',
      'make an image',
      'make a picture',
      'make a photo',
      'genera una imagen',
      'crée une image',
      'cree une image',
      'görsel oluştur',
      'resim oluştur',
      'zeichne eine',
      'zeichne ein',
      'zeichne mir',
      'male eine',
      'male ein',
      'male mir',
    ]) ||
    (hasCreateVerb(lower) &&
      containsAny(lower, [
        'bild',
        'image',
        'picture',
        'photo',
        'foto',
        'zeichnung',
        'imagen',
        'görsel',
        'resim',
      ]))
  )
}

function isAudioRequest(lower: string): boolean {
  return (
    lower.startsWith('sprich:') ||
    lower.startsWith('sprich ') ||
    containsAny(lower, [
      'text to speech',
      'text-to-speech',
      'lies vor',
      'vorlesen',
      'sprachausgabe',
      'generate audio',
      'create audio',
      'make a sound',
      'erzeuge ein audio',
      'erstelle ein audio',
      'genera un audio',
      'crée un audio',
      'ses oluştur',
    ]) ||
    (hasCreateVerb(lower) && hasToken(lower, ['audio', 'tts', 'mp3', 'wav', 'sound']))
  )
}

function isVideoRequest(lower: string): boolean {
  return (
    containsAny(lower, [
      'generate a video',
      'create a video',
      'make a video',
      'erzeuge ein video',
      'erstelle ein video',
      'genera un vídeo',
      'genera un video',
      'crée une vidéo',
      'video oluştur',
    ]) ||
    (hasCreateVerb(lower) && hasToken(lower, ['video', 'film', 'clip']))
  )
}

function isDocumentRequest(lower: string): boolean {
  if (isHowQuestion(lower)) {
    return false
  }
  return (
    containsAny(lower, [
      'generate a document',
      'create a document',
      'write a document',
      'write a report',
      'erstelle ein dokument',
      'erzeuge ein dokument',
      'schreib ein dokument',
      'genera un documento',
      'crée un document',
      'belge oluştur',
    ]) ||
    (hasCreateVerb(lower) &&
      containsAny(lower, [
        'ein dokument',
        'das dokument',
        'a document',
        'the document',
        'docx',
        'xlsx',
        'pptx',
        '.pdf',
        'eine präsentation',
        'a presentation',
        'eine tabelle',
        'a spreadsheet',
      ]))
  )
}

function isHowQuestion(lower: string): boolean {
  return (
    lower.startsWith('how ') ||
    lower.startsWith('wie ') ||
    lower.startsWith('cómo ') ||
    lower.startsWith('como ') ||
    lower.startsWith('comment ') ||
    lower.startsWith('nasıl ')
  )
}

function hasCreateVerb(lower: string): boolean {
  return containsAny(lower, [
    'generate',
    'create',
    'make',
    'draw',
    'paint',
    'write',
    'erzeuge',
    'erstelle',
    'zeichne',
    'male',
    'schreib',
    'zeige',
    'zeig',
    'genera',
    'crée',
    'cree',
    'oluştur',
    'yaz',
  ])
}

function containsAny(haystack: string, needles: string[]): boolean {
  return needles.some((n) => haystack.includes(n))
}

function hasToken(haystack: string, tokens: string[]): boolean {
  return haystack.split(/[^a-z0-9äöüß]+/i).some((word) => tokens.includes(word))
}
