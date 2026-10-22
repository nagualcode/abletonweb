<script lang="ts">
  import { live, type LiveState } from './live.svelte.ts';
  import Meter from './Meter.svelte';
  import TrackButtons from './TrackButtons.svelte';
  import { beatsPerBar, textOn, visibleTracks } from './util.ts';

  let { data }: { data: LiveState } = $props();

  let pxPerBeat = $state(12);
  let follow = $state(true);
  let scroller: HTMLDivElement;

  const tracks = $derived(visibleTracks(data));
  const bar = $derived(beatsPerBar(data));
  const time = $derived(live.tick?.time ?? 0);
  const lastClipEnd = $derived(Math.max(0, ...tracks.flatMap((t) => t.arrangementClips.map((c) => c.end))));
  const totalBeats = $derived(Math.ceil((Math.max(data.song.songLength, lastClipEnd, time) + bar * 16) / bar) * bar);
  const barStep = $derived(pxPerBeat * bar < 28 ? 4 : pxPerBeat * bar < 56 ? 2 : 1);

  // Follow the playhead like Live's Follow: page when it leaves the view.
  $effect(() => {
    if (!follow || !data.song.isPlaying || !scroller) return;
    const x = time * pxPerBeat;
    const { scrollLeft, clientWidth } = scroller;
    if (x < scrollLeft || x > scrollLeft + clientWidth - 40) scroller.scrollLeft = x - 40;
  });

  function zoom(factor: number) {
    const center = (scroller.scrollLeft + scroller.clientWidth / 2) / pxPerBeat;
    pxPerBeat = Math.max(2, Math.min(80, pxPerBeat * factor));
    requestAnimationFrame(() => (scroller.scrollLeft = center * pxPerBeat - scroller.clientWidth / 2));
  }

  function seek(e: MouseEvent) {
    const rect = (e.currentTarget as HTMLElement).getBoundingClientRect();
    const beat = (e.clientX - rect.left) / pxPerBeat;
    live.cmd('set_song', { prop: 'current_song_time', value: Math.max(0, Math.round(beat / bar) * bar) });
  }
</script>

<div class="arr" style:--ppb="{pxPerBeat}px">
  <div class="tools">
    <button onclick={() => zoom(1 / 1.5)}>−</button>
    <button onclick={() => zoom(1.5)}>+</button>
    <button class:on={follow} onclick={() => (follow = !follow)}>Follow</button>
    <button onclick={() => live.cmd('set_song', { prop: 'current_song_time', value: 0 })}>⏮</button>
  </div>
  <div class="body">
    <div class="heads">
      <div class="ruler-pad"></div>
      {#each tracks as t (t.ref)}
        <div class="head" class:selected={data.song.selectedTrack === t.ref}>
          <button class="name" style:background={t.color} style:color={textOn(t.color)}
            onclick={() => live.cmd('select_track', { track: t.ref })}>
            {t.group ? '  ' : ''}{#if t.isFoldable}<span role="presentation" onclick={(e) => { e.stopPropagation(); live.cmd('set_track', { track: t.ref, prop: 'folded', value: !t.folded }); }}>{t.folded ? '▸ ' : '▾ '}</span>{/if}{t.name}
          </button>
          <div class="row">
            <TrackButtons track={t} monitor={false} />
            <div class="m"><Meter value={live.meter(t.ref)} vertical={false} /></div>
          </div>
        </div>
      {/each}
    </div>
    <div class="scroller" bind:this={scroller}>
      <div class="content" style:width="{totalBeats * pxPerBeat}px">
        <div class="ruler" role="presentation" onclick={seek}>
          {#if data.song.loop}
            <div class="loop" style:left="{data.song.loopStart * pxPerBeat}px" style:width="{data.song.loopLength * pxPerBeat}px"></div>
          {/if}
          {#each { length: Math.ceil(totalBeats / bar / barStep) } as _, i (i)}
            <span class="bar" style:left="{i * barStep * bar * pxPerBeat}px">{i * barStep + 1}</span>
          {/each}
        </div>
        {#each tracks as t (t.ref)}
          <div class="lane" style:--bar="{bar * pxPerBeat}px">
            {#each t.arrangementClips as c, i (i)}
              <div class="clip" style:left="{c.start * pxPerBeat}px" style:width="{Math.max(2, (c.end - c.start) * pxPerBeat)}px"
                style:background={c.color} style:color={textOn(c.color)}>{c.name}</div>
            {/each}
          </div>
        {/each}
        <div class="playhead" style:transform="translateX({time * pxPerBeat}px)"></div>
      </div>
    </div>
  </div>
</div>

<style>
  .arr {
    --lane: 52px;
    --ruler: 26px;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .tools {
    display: flex;
    gap: 4px;
    padding: 6px 8px;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
  }
  .tools .on {
    background: var(--accent);
    color: var(--bg);
  }
  .body {
    flex: 1;
    min-height: 0;
    display: flex;
    overflow-y: auto;
    overscroll-behavior: contain;
  }
  .heads {
    flex: none;
    width: 170px;
    background: var(--panel);
    border-right: 1px solid var(--line);
    position: sticky;
    left: 0;
    z-index: 2;
    height: max-content;
  }
  .ruler-pad {
    height: var(--ruler);
    border-bottom: 1px solid var(--line);
  }
  .head {
    height: var(--lane);
    border-bottom: 1px solid var(--line);
    padding: 3px 4px;
    display: flex;
    flex-direction: column;
    gap: 3px;
  }
  .head.selected {
    background: var(--panel-2);
  }
  .name {
    border-radius: 3px;
    padding: 2px 6px;
    font-size: 12px;
    font-weight: 600;
    text-align: left;
    white-space: pre;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 4px;
  }
  .row :global(.buttons) {
    flex-wrap: nowrap;
  }
  .row :global(button) {
    min-width: 24px;
    padding: 2px 3px;
    font-size: 11px;
  }
  .m {
    flex: 1;
    height: 10px;
  }
  .scroller {
    flex: 1;
    overflow-x: auto;
    overflow-y: hidden;
    height: max-content;
  }
  .content {
    position: relative;
  }
  .ruler {
    height: var(--ruler);
    position: sticky;
    top: 0;
    background: var(--panel);
    border-bottom: 1px solid var(--line);
    cursor: pointer;
  }
  .bar {
    position: absolute;
    top: 0;
    bottom: 0;
    padding: 5px 3px;
    font-size: 11px;
    color: var(--muted);
    border-left: 1px solid var(--line);
  }
  .loop {
    position: absolute;
    top: 0;
    height: 6px;
    background: var(--muted-2);
  }
  .lane {
    position: relative;
    height: var(--lane);
    border-bottom: 1px solid var(--line);
    background-image: linear-gradient(to right, var(--line) 1px, transparent 1px);
    background-size: var(--bar) 100%;
  }
  .clip {
    position: absolute;
    top: 3px;
    bottom: 3px;
    border-radius: 3px;
    font-size: 11px;
    padding: 2px 4px;
    overflow: hidden;
    white-space: nowrap;
    box-shadow: inset 0 0 0 1px #0005;
  }
  .playhead {
    position: absolute;
    top: 0;
    bottom: 0;
    left: 0;
    width: 1px;
    background: var(--text);
    pointer-events: none;
    transition: transform 100ms linear;
  }
</style>
