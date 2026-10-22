import type { LiveState, Track } from './live.svelte.ts';

export const DEFAULT_VOLUME = 0.85; // 0 dB in Live's volume parameter

export function textOn(color: string | null): string {
  if (!color) return 'var(--text)';
  const n = parseInt(color.slice(1), 16);
  const r = (n >> 16) & 255, g = (n >> 8) & 255, b = n & 255;
  return 0.299 * r + 0.587 * g + 0.114 * b > 150 ? '#111' : '#fff';
}

export function beatsPerBar(s: LiveState): number {
  return s.song.signatureNumerator * (4 / s.song.signatureDenominator);
}

// Live-style "bar.beat.sixteenth" for a song time in beats.
export function formatPosition(beats: number, s: LiveState): string {
  const beatLen = 4 / s.song.signatureDenominator;
  const inBeats = beats / beatLen;
  const bar = Math.floor(inBeats / s.song.signatureNumerator) + 1;
  const beat = Math.floor(inBeats % s.song.signatureNumerator) + 1;
  const sixteenth = Math.floor((beats % beatLen) / (beatLen / 4)) + 1;
  return `${bar}.${beat}.${sixteenth}`;
}

export function formatSeconds(seconds: number): string {
  const s = Math.max(0, Math.floor(seconds));
  return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}`;
}

export function sceneLabel(s: LiveState, index: number): string {
  return s.scenes[index]?.name || String(index + 1);
}

export function visibleTracks(s: LiveState): Track[] {
  return s.tracks.filter((t) => t.isVisible);
}

export function recordingSlot(t: Track): number {
  return t.slots.findIndex((sl) => sl.state === 'recording');
}

export function monitorLabel(m: Track['monitor']): string {
  return m === 'in' ? 'In' : m === 'auto' ? 'Auto' : 'Off';
}

export type View = 'session' | 'arrangement' | 'record' | 'mixer';
