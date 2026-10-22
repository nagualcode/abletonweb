<script lang="ts">
  import { live, type LiveState, type Track } from './live.svelte.ts';
  import MixerStrip from './MixerStrip.svelte';
  import { sceneLabel, textOn, visibleTracks } from './util.ts';

  let { data }: { data: LiveState } = $props();

  let grid: HTMLDivElement;
  let mixer: HTMLDivElement;

  const tracks = $derived(visibleTracks(data));
  const columns = $derived(`repeat(${tracks.length + data.returns.length}, var(--col)) var(--master-col)`);

  // Keep both panes horizontally aligned.
  function sync(from: HTMLElement | undefined, to: HTMLElement | undefined) {
    if (from && to && to.scrollLeft !== from.scrollLeft) to.scrollLeft = from.scrollLeft;
  }

  function fireSlot(track: Track, scene: number) {
    live.cmd('fire_slot', { track: track.ref, scene });
  }

  function headerTap(track: Track) {
    live.cmd('select_track', { track: track.ref });
  }

  function fold(track: Track, e: Event) {
    e.stopPropagation();
    live.cmd('set_track', { track: track.ref, prop: 'folded', value: !track.folded });
  }
</script>

<div class="session">
  <div class="grid" bind:this={grid} onscroll={() => sync(grid, mixer)} style:grid-template-columns={columns}>
    {#each tracks as t (t.ref)}
      <button class="head" class:selected={data.song.selectedTrack === t.ref}
        style:background={t.color} style:color={textOn(t.color)} onclick={() => headerTap(t)}>
        {#if t.isFoldable}
          <span class="fold" role="presentation" onclick={(e) => fold(t, e)}>{t.folded ? '▸' : '▾'}</span>
        {/if}
        <span class="name">{t.group ? '  ' : ''}{t.name}</span>
      </button>
    {/each}
    {#each data.returns as t (t.ref)}
      <div class="head ret" style:background={t.color} style:color={textOn(t.color)}><span class="name">{t.name}</span></div>
    {/each}
    <div class="head master sticky">Master</div>

    {#each data.scenes as scene, si (si)}
      {#each tracks as t (t.ref)}
        {@const slot = t.slots[si]}
        {@const pos = live.tick?.clipPos[t.ref]}
        <button
          class="slot {slot?.state ?? 'empty'}"
          class:has={slot?.clip}
          class:armed={t.arm}
          class:selscene={data.song.selectedScene === si}
          style:--clip={slot?.clip?.color ?? 'transparent'}
          onclick={() => fireSlot(t, si)}
        >
          {#if slot?.clip}
            <span class="ind"></span>
            <span class="clipname" style:color={textOn(slot.clip.color)}>{slot.clip.name}</span>
            {#if slot.state === 'playing' && t.playingSlot === si && pos !== undefined}
              <span class="progress" style:--p={Math.max(0, Math.min(1, pos))}></span>
            {/if}
          {:else if slot?.group}
            <span class="ind group"></span>
          {:else if t.arm}
            <span class="ind rec-empty"></span>
          {:else if slot?.hasStop}
            <span class="ind stop"></span>
          {/if}
        </button>
      {/each}
      {#each data.returns as t (t.ref)}
        <div class="slot void"></div>
      {/each}
      <button class="slot scene sticky" class:triggered={scene.triggered} class:selscene={data.song.selectedScene === si}
        style:--clip={scene.color ?? 'transparent'} onclick={() => live.cmd('fire_scene', { scene: si })}>
        <span class="ind play"></span><span class="clipname">{sceneLabel(data, si)}</span>
      </button>
    {/each}

    {#each tracks as t (t.ref)}
      <button class="slot stoprow" onclick={() => live.cmd('stop_track', { track: t.ref })} title="Stop">
        <span class="ind stop"></span>
      </button>
    {/each}
    {#each data.returns as t (t.ref)}
      <div class="slot void"></div>
    {/each}
    <button class="slot stoprow sticky" onclick={() => live.cmd('stop_all_clips')}>Stop All</button>
  </div>

  <div class="mixer" bind:this={mixer} onscroll={() => sync(mixer, grid)} style:grid-template-columns={columns}>
      {#each tracks as t (t.ref)}
        <div class="col" class:selected={data.song.selectedTrack === t.ref}><MixerStrip track={t} /></div>
      {/each}
      {#each data.returns as t (t.ref)}
        <div class="col"><MixerStrip track={t} /></div>
      {/each}
      <div class="col sticky"><MixerStrip track={data.master} /></div>
  </div>
</div>

<style>
  .session {
    --col: 104px;
    --master-col: 120px;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .grid {
    /* Only as tall as its scenes; the mixer takes the remaining space. */
    flex: 0 1 auto;
    min-height: 120px;
    width: fit-content;
    border-bottom: 2px solid var(--line);
    max-width: 100%;
    overflow: auto;
    display: grid;
    grid-auto-rows: 34px;
    gap: 1px;
    background: var(--line);
    align-content: start;
    overscroll-behavior: contain;
  }
  .sticky {
    position: sticky;
    right: 0;
    z-index: 1;
  }
  .head {
    position: sticky;
    top: 0;
    z-index: 2;
    display: flex;
    align-items: center;
    gap: 4px;
    padding: 0 6px;
    border-radius: 0;
    font-size: 12px;
    font-weight: 600;
    text-align: left;
    background: var(--btn);
  }
  .head.master {
    z-index: 3;
    background: var(--panel-2);
  }
  .head.selected {
    box-shadow: inset 0 -3px 0 var(--text);
  }
  .fold {
    padding: 0 2px;
  }
  .name,
  .clipname {
    overflow: hidden;
    white-space: pre;
    text-overflow: ellipsis;
    font-size: 12px;
  }
  .slot {
    position: relative;
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 0 6px;
    border-radius: 0;
    background: var(--slot);
    text-align: left;
    overflow: hidden;
  }
  .slot.selscene {
    background: var(--slot-sel);
  }
  .slot.has {
    background: var(--clip);
  }
  .slot.void {
    background: var(--bg);
  }
  .ind {
    flex: none;
    width: 0;
    height: 0;
    border-style: solid;
    border-width: 6px 0 6px 10px;
    border-color: transparent transparent transparent #000a;
  }
  .ind.stop {
    width: 9px;
    height: 9px;
    border: 0;
    background: var(--muted-2);
  }
  .ind.rec-empty {
    width: 11px;
    height: 11px;
    border: 0;
    border-radius: 50%;
    background: #ff3b3066;
  }
  .ind.group {
    border-color: transparent transparent transparent var(--muted);
  }
  .playing .ind {
    border-left-color: var(--play);
  }
  .playing {
    box-shadow: inset 0 0 0 2px var(--play);
  }
  .triggered .ind {
    animation: blink 0.5s steps(1) infinite;
  }
  .triggered {
    box-shadow: inset 0 0 0 2px var(--play);
    animation: blink 0.5s steps(1) infinite;
  }
  .recording {
    box-shadow: inset 0 0 0 2px var(--rec);
  }
  .recording .ind,
  .triggered.armed .ind.rec-empty {
    width: 11px;
    height: 11px;
    border: 0;
    border-radius: 50%;
    background: var(--rec);
  }
  .group.playing .ind,
  .scene .ind.play {
    border-left-color: var(--muted);
  }
  .progress {
    position: absolute;
    left: 0;
    bottom: 0;
    height: 3px;
    width: 100%;
    background: #000a;
    transform-origin: left;
    transform: scaleX(var(--p));
  }
  .scene {
    background: var(--panel-2);
    border-left: 3px solid var(--clip);
  }
  .stoprow {
    justify-content: center;
    background: var(--panel);
    font-size: 12px;
  }
  .mixer {
    flex: 1 0 auto;
    height: min(46vh, 300px);
    width: fit-content;
    max-width: 100%;
    display: grid;
    gap: 1px;
    background: var(--line);
    overflow-x: auto;
    overflow-y: hidden;
  }
  .col {
    background: var(--panel);
    min-height: 0;
  }
  .col.selected {
    background: var(--panel-2);
  }
  @keyframes blink {
    50% {
      opacity: 0.45;
    }
  }
</style>
