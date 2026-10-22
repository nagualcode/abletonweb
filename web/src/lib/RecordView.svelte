<script lang="ts">
  import Fader from './Fader.svelte';
  import { live, type LiveState, type Track } from './live.svelte.ts';
  import Meter from './Meter.svelte';
  import { DEFAULT_VOLUME, formatPosition, formatSeconds, monitorLabel, recordingSlot, textOn } from './util.ts';

  // Simplified screen for recording yourself away from the computer:
  // pick a track, check the input level, hit one big button.
  let { data }: { data: LiveState } = $props();

  type Mode = 'session' | 'arrangement';
  const MODE_KEY = 'abletonweb.recordMode';
  let mode = $state<Mode>(loadMode());
  let startedAt = $state<number | null>(null);
  let busy = $state(false);

  function loadMode(): Mode {
    try {
      return localStorage.getItem(MODE_KEY) === 'arrangement' ? 'arrangement' : 'session';
    } catch {
      return 'session';
    }
  }

  function setMode(m: Mode) {
    mode = m;
    try {
      localStorage.setItem(MODE_KEY, m);
    } catch { /* ignore */ }
  }

  const armable = $derived(data.tracks.filter((t) => t.canArm));
  const selected = $derived<Track | null>(
    armable.find((t) => t.ref === data.song.selectedTrack) ?? armable.find((t) => t.arm) ?? armable[0] ?? null);
  const recSlot = $derived(selected ? recordingSlot(selected) : -1);
  const pendingSlot = $derived(selected && selected.arm ? selected.slots.findIndex((s) => s.state === 'triggered' && !s.clip) : -1);
  const recording = $derived(mode === 'session'
    ? recSlot >= 0
    : data.song.recordMode && data.song.isPlaying);
  const waiting = $derived(!recording && (mode === 'session' ? pendingSlot >= 0 : data.song.isCountingIn === true));
  const time = $derived(live.tick?.time ?? 0);
  const inputMeter = $derived(selected ? (live.tick?.inputMeters[selected.ref] ?? live.meter(selected.ref)) : [0, 0] as [number, number]);
  const elapsed = $derived(startedAt !== null ? ((time - startedAt) * 60) / data.song.tempo : 0);

  $effect(() => {
    if (recording && startedAt === null && live.tick) startedAt = time;
    if (!recording) startedAt = null;
  });

  function select(t: Track) {
    live.cmd('select_track', { track: t.ref });
  }

  async function run(fn: () => Promise<unknown>) {
    if (busy) return;
    busy = true;
    navigator.vibrate?.(30);
    try {
      await fn();
    } catch { /* already shown by live.cmd */ }
    busy = false;
  }

  function toggleRecord() {
    const t = selected;
    if (!t) return;
    run(async () => {
      if (mode === 'session') {
        if (recording || waiting) await live.cmd('stop_track', { track: t.ref });
        else await live.cmd('record_into_track', { track: t.ref });
      } else if (recording || waiting) {
        await live.cmd('stop');
        await live.cmd('record', { value: false });
      } else {
        if (!t.arm) await live.cmd('set_track', { track: t.ref, prop: 'arm', value: true, exclusive: true });
        await live.cmd('record', { value: true });
      }
    });
  }

  function set(prop: string, value: unknown) {
    if (selected) live.cmd('set_track', { track: selected.ref, prop, value, exclusive: true });
  }
</script>

<div class="rec">
  <div class="chips">
    {#each armable as t (t.ref)}
      <button class="chip" class:sel={selected?.ref === t.ref} onclick={() => select(t)}>
        <span class="dot" style:background={t.color}></span>
        <span class="cn">{t.name}</span>
        {#if t.arm}<span class="armed">●</span>{/if}
      </button>
    {/each}
  </div>

  {#if selected}
    <section class="card">
      <div class="title" style:background={selected.color} style:color={textOn(selected.color)}>
        <span>{selected.name}</span>
        <span class="input">{selected.input ?? ''}</span>
      </div>

      <div class="label">Input</div>
      <div class="inmeter"><Meter value={inputMeter} vertical={false} /></div>

      <div class="controls">
        <button class="arm" class:on={selected.arm} onclick={() => set('arm', !selected.arm)}>● Arm</button>
        <div class="seg">
          {#each ['in', 'auto', 'off'] as const as m (m)}
            <button class:on={selected.monitor === m} onclick={() => set('monitor', m)}>{monitorLabel(m)}</button>
          {/each}
        </div>
      </div>

      <div class="label">Volume <span class="val">{selected.volumeStr}</span></div>
      <Fader vertical={false} value={selected.volume} defaultValue={DEFAULT_VOLUME}
        onchange={(v) => live.cmdThrottled(`${selected.ref}.volume`, 'set_track', { track: selected.ref, prop: 'volume', value: v })} />
    </section>

    <div class="seg mode">
      <button class:on={mode === 'session'} onclick={() => setMode('session')}>New clip (Session)</button>
      <button class:on={mode === 'arrangement'} onclick={() => setMode('arrangement')}>Arrangement</button>
    </div>

    <div class="big">
      <button class="recbtn" class:recording class:waiting disabled={busy && !recording} onclick={toggleRecord}
        aria-label={recording ? 'Stop recording' : 'Record'}>
        <span class="shape"></span>
      </button>
      <div class="status">
        {#if recording}
          <strong class="red">RECORDING {formatSeconds(elapsed)}</strong>
          <span>{mode === 'session' ? (data.scenes[recSlot]?.name || `Scene ${recSlot + 1}`) : formatPosition(time, data)}</span>
        {:else if waiting}
          <strong>Waiting to start…</strong>
        {:else}
          <strong>{mode === 'session' ? 'Tap to record a new clip' : 'Tap to record into the Arrangement'}</strong>
          <span>{formatPosition(time, data)} · {data.song.tempo.toFixed(1)} BPM</span>
        {/if}
      </div>
    </div>

    <div class="row">
      <button class="wide" class:on={data.song.isPlaying} onclick={() => live.cmd(data.song.isPlaying ? 'stop' : 'play')}>
        {data.song.isPlaying ? '■ Stop' : '▶ Play'}
      </button>
      <button class="wide" class:on={data.song.metronome} onclick={() => live.cmd('set_song', { prop: 'metronome', value: !data.song.metronome })}>Metronome</button>
      <button class="wide" disabled={!data.song.canUndo || recording} onclick={() => live.cmd('undo')}>↶ Undo</button>
    </div>
  {:else}
    <p class="empty">No audio/MIDI tracks in this set.</p>
  {/if}
</div>

<style>
  .rec {
    display: flex;
    flex-direction: column;
    gap: 14px;
    padding: 12px;
    padding-left: max(12px, env(safe-area-inset-left));
    padding-right: max(12px, env(safe-area-inset-right));
    padding-bottom: max(16px, env(safe-area-inset-bottom));
    max-width: 560px;
    width: 100%;
    margin: 0 auto;
    overflow-y: auto;
    height: 100%;
  }
  .chips {
    display: flex;
    gap: 6px;
    overflow-x: auto;
    padding-bottom: 2px;
    flex: none;
  }
  .chip {
    flex: none;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 8px 12px;
    border-radius: 18px;
    max-width: 180px;
  }
  .chip.sel {
    background: var(--text);
    color: var(--bg);
  }
  .cn {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
  .dot {
    width: 10px;
    height: 10px;
    border-radius: 50%;
    flex: none;
  }
  .armed {
    color: var(--rec);
  }
  .card {
    background: var(--panel);
    border-radius: 10px;
    padding: 0 12px 14px;
    display: flex;
    flex-direction: column;
    gap: 8px;
    overflow: hidden;
    flex: none;
  }
  .title {
    margin: 0 -12px 4px;
    padding: 10px 12px;
    display: flex;
    justify-content: space-between;
    font-weight: 600;
  }
  .input {
    font-weight: 400;
    opacity: 0.8;
    font-size: 13px;
  }
  .label {
    font-size: 12px;
    color: var(--muted);
    display: flex;
    justify-content: space-between;
  }
  .val {
    font-variant-numeric: tabular-nums;
  }
  .inmeter {
    height: 22px;
  }
  .controls {
    display: flex;
    gap: 8px;
  }
  .arm {
    padding: 10px 14px;
  }
  .arm.on {
    background: var(--rec);
    color: #fff;
  }
  .seg {
    display: flex;
    flex: 1;
    background: var(--bg);
    border-radius: 6px;
    padding: 2px;
    gap: 2px;
  }
  .seg button {
    flex: 1;
    background: transparent;
    padding: 8px 4px;
  }
  .seg button.on {
    background: var(--btn-on);
    color: var(--text);
  }
  .mode {
    flex: none;
  }
  .big {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 10px;
    padding: 8px 0;
    flex: none;
  }
  .recbtn {
    width: 132px;
    height: 132px;
    border-radius: 50%;
    border: 5px solid var(--rec);
    background: var(--panel);
    display: grid;
    place-items: center;
  }
  .shape {
    width: 76px;
    height: 76px;
    border-radius: 50%;
    background: var(--rec);
    transition: all 150ms;
  }
  .recbtn.recording {
    animation: pulse 1.2s ease-in-out infinite;
  }
  .recbtn.recording .shape {
    width: 48px;
    height: 48px;
    border-radius: 8px;
  }
  .recbtn.waiting .shape {
    animation: blink 0.5s steps(1) infinite;
  }
  .status {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 2px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }
  .status span {
    color: var(--muted);
    font-size: 13px;
  }
  .red {
    color: var(--rec);
  }
  .row {
    display: flex;
    gap: 8px;
    flex: none;
  }
  .wide {
    flex: 1;
    padding: 14px 6px;
  }
  .wide.on {
    background: var(--btn-on);
  }
  .empty {
    color: var(--muted);
    text-align: center;
  }
  @keyframes pulse {
    50% {
      box-shadow: 0 0 0 10px #ff3b3033;
    }
  }
  @keyframes blink {
    50% {
      opacity: 0.3;
    }
  }
</style>
