import { describe, expect, it } from 'vitest'
import { classifyGenerationLocal } from '@/lib/classifyGeneration'

describe('classifyGenerationLocal', () => {
  it('routes picture requests and leaves ordinary questions alone', () => {
    expect(classifyGenerationLocal('ein echtes bild einer katze')).toBe('image')
    expect(classifyGenerationLocal('Generate an image of a cat')).toBe('image')
    expect(classifyGenerationLocal('/pic a red balloon')).toBe('image')
    expect(classifyGenerationLocal('zeichne eine katze')).toBe('image')
    expect(classifyGenerationLocal('male mir einen hund')).toBe('image')
    expect(classifyGenerationLocal('und wer ist jetzt mats?')).toBeNull()
    expect(classifyGenerationLocal('What is in the picture I uploaded?')).toBeNull()
    expect(classifyGenerationLocal('How do I write a report?')).toBeNull()
  })

  it('routes audio, video, and document create-requests', () => {
    expect(classifyGenerationLocal('Sprich: Guten Morgen')).toBe('audio')
    expect(classifyGenerationLocal('Erstelle ein Audio von diesem Text')).toBe('audio')
    expect(classifyGenerationLocal('Erstelle ein Dokument über Mats')).toBe('document')
    expect(classifyGenerationLocal('Create a video of waves')).toBe('video')
  })
})
