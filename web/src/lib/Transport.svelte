<script lang="ts">
  import { live } from './live.svelte.ts';
  import { formatPosition, type View } from './util.ts';

  let { view = $bindable(), views, compact = false }: {
    view: View;
    views: { id: View; label: string }[];
    compact?: boolean;
  } = $props();

  const song = $derived(live.state?.song);
  const position = $derived(live.state && live.tick ? formatPosition(live.tick.time, live.state) : '1.1.1');

  function setSong(prop: string, value: unknown) {
    live.cmd('set_song', { prop, value });
  }
</script>

<header class="transport" class:compact>
  <nav class="views">
    {#each views as v (v.id)}
      <button class:on={view === v.id} onclick={() => (view = v.id)}>{v.label}</button>
    {/each}
  </nav>

  {#if song}
    <div class="group">
      <button class="icon" class:play={song.isPlaying} title="Play" onclick={() => live.cmd('play')}>
        <svg viewBox="0 0 20 20"><path d="M5 3l12 7-12 7z" /></svg>
      </button>
      <button class="icon" title="Stop" onclick={() => live.cmd('stop')}>
        <svg viewBox="0 0 20 20"><rect x="4" y="4" width="12" height="12" /></svg>
      </button>
      <button class="icon" class:rec={song.recordMode} title="Arrangement Record" onclick={() => live.cmd('record')}>
        <svg viewBox="0 0 20 20"><circle cx="10" cy="10" r="6" /></svg>
      </button>
      {#if !compact}
        <button class="icon" class:rec={song.sessionRecord} title="Session Record" onclick={() => live.cmd('session_record')}>
          <svg viewBox="0 0 20 20"><circle cx="10" cy="10" r="6" fill="none" stroke-width="2.5" /><path d="M10 7v6M7 10h6" stroke-width="2" /></svg>
        </button>
        <button class="text" class:on={song.arrangementOverdub} title="Arrangement Overdub" onclick={() => setSong('arrangement_overdub', !song.arrangementOverdub)}>OVR</button>
      {/if}
    </div>

    <div class="group">
      <span class="pos">{position}</span>
      <span class="tempo">{song.tempo.toFixed(2)}</span>
      {#if !compact}
        <button class="text" title="Tap Tempo" onclick={() => live.cmd('tap_tempo')}>TAP</button>
      {/if}
      <button class="icon" class:on={song.metronome} title="Metronome" onclick={() => setSong('metronome', !song.metronome)}>
        <svg viewBox="0 0 20 20"><path d="M7 2h6l3 16H4z" fill="none" stroke-width="1.6" /><path d="M10 13l5-8" stroke-width="1.6" /></svg>
      </button>
      {#if song.loop !== undefined && !compact}
        <button class="text" class:on={song.loop} title="Loop" onclick={() => setSong('loop', !song.loop)}>LOOP</button>
      {/if}
      {#if song.backToArranger && !compact}
        <button class="text warn" title="Back to Arrangement" onclick={() => setSong('back_to_arranger', false)}>◀ ARR</button>
      {/if}
    </div>

    <div class="group">
      <button class="text" disabled={!song.canUndo} onclick={() => live.cmd('undo')} title="Undo">↶</button>
      {#if !compact}
        <button class="text" disabled={!song.canRedo} onclick={() => live.cmd('redo')} title="Redo">↷</button>
      {/if}
    </div>
  {/if}

  <span class="status {live.liveConnected ? 'ok' : live.socket === 'open' ? 'warn' : 'bad'}"
    title={live.liveConnected ? `Live ${live.version}` : live.socket === 'open' ? 'Bridge ok, Live not connected' : 'Offline'}></span>
</header>

<style>
  .transport {
    display: flex;
    align-items: center;
    gap: 14px;
    padding: 8px 12px;
    padding-top: max(8px, env(safe-area-inset-top));
    background: var(--panel);
    border-bottom: 1px solid var(--line);
    flex-wrap: wrap;
  }
  .transport.compact {
    gap: 8px;
    padding-left: max(12px, env(safe-area-inset-left));
    padding-right: max(12px, env(safe-area-inset-right));
  }
  .group {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .views {
    display: flex;
    background: var(--bg);
    border-radius: 6px;
    padding: 2px;
  }
  .views button {
    border: 0;
    background: transparent;
    padding: 6px 12px;
    border-radius: 4px;
    color: var(--muted);
  }
  .views button.on {
    background: var(--btn);
    color: var(--text);
  }
  .compact .views {
    order: 10;
    width: 100%;
  }
  .compact .views button {
    flex: 1;
    padding: 8px;
  }
  .icon svg {
    width: 18px;
    height: 18px;
    fill: currentColor;
    stroke: currentColor;
  }
  .icon.play {
    color: var(--play);
  }
  .icon.rec {
    color: var(--rec);
  }
  .icon.on,
  .text.on {
    color: var(--bg);
    background: var(--accent);
  }
  .text.warn {
    background: var(--accent);
    color: var(--bg);
  }
  .pos,
  .tempo {
    font-variant-numeric: tabular-nums;
    background: var(--bg);
    padding: 6px 10px;
    border-radius: 4px;
    min-width: 64px;
    text-align: center;
  }
  .status {
    margin-left: auto;
    width: 10px;
    height: 10px;
    border-radius: 50%;
  }
  .status.ok {
    background: var(--play);
  }
  .status.warn {
    background: var(--accent);
  }
  .status.bad {
    background: var(--rec);
  }
</style>
