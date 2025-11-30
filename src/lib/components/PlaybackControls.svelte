<script>
  export let compact = false;
  export let keybindings = { pause_resume: "F9", previous: "F10", next: "F11" };
  import Icon from '@iconify/svelte';

  import {
    isPlaying,
    isPaused,
    loopMode,
    shuffleMode,
    pauseResume,
    stopPlayback,
    playNext,
    playPrevious,
    toggleLoop,
    toggleShuffle
  } from '../stores/player.js';
</script>

<div class="flex items-center justify-center gap-4 {compact ? 'py-2' : ''}">
  <!-- Shuffle -->
  <button
    class="transition-colors {$shuffleMode ? 'text-[#1db954]' : 'text-white/40 hover:text-white'}"
    on:click={toggleShuffle}
    title="Shuffle"
  >
    <Icon icon="mdi:shuffle" class="w-4 h-4" />
  </button>

  <!-- Previous -->
  <button
    class="text-white/60 hover:text-white transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
    on:click={playPrevious}
    disabled={!$isPlaying}
    title="Previous ({keybindings.previous})"
  >
    <Icon icon="mdi:skip-previous" class="w-5 h-5" />
  </button>

  <!-- Play/Pause -->
  <button
    class="w-8 h-8 rounded-full bg-white hover:scale-105 transition-transform flex items-center justify-center"
    on:click={pauseResume}
    title="Play/Pause ({keybindings.pause_resume})"
  >
    {#if $isPlaying && !$isPaused}
      <Icon icon="mdi:pause" class="w-5 h-5 text-black" />
    {:else}
      <Icon icon="mdi:play" class="w-5 h-5 text-black" />
    {/if}
  </button>

  <!-- Next -->
  <button
    class="text-white/60 hover:text-white transition-colors disabled:opacity-30 disabled:cursor-not-allowed"
    on:click={playNext}
    disabled={!$isPlaying}
    title="Next ({keybindings.next})"
  >
    <Icon icon="mdi:skip-next" class="w-5 h-5" />
  </button>

  <!-- Loop -->
  <button
    class="transition-colors {$loopMode ? 'text-[#1db954]' : 'text-white/40 hover:text-white'}"
    on:click={toggleLoop}
    title="Loop"
  >
    <Icon icon="mdi:repeat" class="w-4 h-4" />
  </button>
</div>
