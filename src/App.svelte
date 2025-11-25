<script>
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { fade, fly } from "svelte/transition";
  import Icon from "@iconify/svelte";
  import appIcon from "./icon.png";

  import Header from "./lib/components/Header.svelte";
  import MidiFileList from "./lib/components/MidiFileList.svelte";
  import PlaybackControls from "./lib/components/PlaybackControls.svelte";
  import Timeline from "./lib/components/Timeline.svelte";
  import PlaylistManager from "./lib/components/PlaylistManager.svelte";
  import FavoritesView from "./lib/components/FavoritesView.svelte";
  import SavedPlaylistsView from "./lib/components/SavedPlaylistsView.svelte";

  import {
    loadMidiFiles,
    initializeListeners,
    isMinimized,
    isDraggable,
    miniMode,
    toggleMiniMode,
    currentFile,
    playlist,
    favorites,
    savedPlaylists,
    smartPause,
    loopMode,
    isPlaying,
    isPaused,
    pauseResume,
    stopPlayback,
    playNext,
    playPrevious,
    toggleLoop,
    toggleDraggable,
  } from "./lib/stores/player.js";

  let activeView = "library"; // "library", "queue", "favorites", "playlists"

  const navItems = [
    { id: "library", icon: "mdi:library-music", label: "Library" },
    {
      id: "queue",
      icon: "mdi:playlist-play",
      label: "Queue",
      badge: () => $playlist.length,
    },
    {
      id: "favorites",
      icon: "mdi:heart",
      label: "Favorites",
      badge: () => $favorites.length,
    },
    {
      id: "playlists",
      icon: "mdi:folder-music",
      label: "Playlists",
      badge: () => $savedPlaylists.length,
    },
  ];

  const shortcuts = [
    { action: "Play / Pause", key: "F9" },
    { action: "Stop", key: "F12 / End" },
    { action: "Previous", key: "F10" },
    { action: "Next", key: "F11" },
    { action: "Toggle Loop", key: "Ctrl + L" },
  ];

  onMount(async () => {
    await loadMidiFiles();
    initializeListeners();

    // Listen for global shortcut events from Rust backend
    const unlisten = await listen("global-shortcut", async (event) => {
      const action = event.payload;
      console.log(`Global shortcut received: ${action}`);

      switch (action) {
        case "pause_resume":
          await pauseResume();
          break;
        case "stop":
          await stopPlayback();
          break;
        case "previous":
          await playPrevious();
          break;
        case "next":
          await playNext();
          break;
        case "toggle_loop":
          await toggleLoop();
          break;
      }
    });

    return () => {
      unlisten();
    };
  });

  const filename = (path) => {
    if (!path) return "No track selected";
    const parts = path.split(/[\\/]/);
    return parts[parts.length - 1] || path;
  };

  // Toggle body/html background and overflow based on mini mode
  $: {
    if (typeof document !== "undefined") {
      if ($miniMode) {
        document.body.style.background = "transparent";
        document.body.style.overflow = "hidden";
        document.body.style.border = "none";
        document.body.style.borderRadius = "0";
        document.documentElement.style.background = "transparent";
        document.documentElement.style.overflow = "hidden";
      } else {
        document.body.style.background = "";
        document.body.style.overflow = "";
        document.documentElement.style.background = "";
        document.documentElement.style.overflow = "";
      }
    }
  }
</script>

{#if $miniMode}
  <!-- Mini Mode - Container with drag handle -->
  <div class="flex flex-col items-center">
    <!-- Drag handle above the icon (same style as main app) -->
    <div
      class="drag-handle flex bg-[#18181893] items-center justify-center cursor-move hover:opacity-80 transition-colors group mb-0.5 px-3 rounded"
      title="Drag to move"
    >
      <Icon
        icon="mdi:drag-horizontal"
        class="w-5 h-5 text-white/20 group-hover:text-white/40 transition-colors"
      />
    </div>

    <!-- Clickable Icon -->
    <button
      class="w-14 h-14 rounded-2xl bg-[#18181893] border border-white/10 shadow-2xl overflow-hidden relative flex items-center justify-center cursor-pointer active:scale-95 transition-transform"
      onclick={toggleMiniMode}
      title="Click to expand"
    >
      <!-- Playing indicator ring -->
      {#if $isPlaying && !$isPaused}
        <div
          class="absolute inset-0 rounded-2xl border-2 border-[#1db954] animate-pulse pointer-events-none"
        ></div>
      {/if}

      <!-- App Icon -->
      <img
        src={appIcon}
        alt="App Icon"
        class="w-10 h-10 rounded-lg pointer-events-none"
      />
    </button>
  </div>
{:else}
  <main class="">
    <div
      class="h-screen w-full flex flex-col overflow-hidden rounded-md {$isDraggable
        ? ''
        : 'pointer-events-none'}"
    >
      {#if !$isMinimized}
        <!-- Spotify-style layout -->
        <div class="flex flex-1 min-h-0 overflow-hidden">
          <!-- Sidebar -->
          <aside
            class="spotify-sidebar w-56 flex flex-col p-4 gap-2 no-drag border-r border-white/5"
          >
            <!-- Drag Handle with Mini Mode Toggle -->
            <div
              class="flex items-center justify-between py-2 -mx-4 -mt-4 mb-2 px-2"
            >
              <div class="w-8"></div>
              <div
                class="drag-handle flex-1 flex items-center justify-center cursor-move hover:bg-white/5 transition-colors group py-1 rounded"
                title="Drag to move window"
              >
                <Icon
                  icon="mdi:drag-horizontal"
                  class="w-6 h-6 text-white/20 group-hover:text-white/40 transition-colors"
                />
              </div>
              <button
                class="w-8 h-8 flex items-center justify-center rounded-lg text-white/40 hover:text-white hover:bg-white/10 transition-all"
                onclick={toggleMiniMode}
                title="Minimize to floating icon"
              >
                <Icon icon="mdi:minus" class="w-5 h-5" />
              </button>
            </div>

            <!-- Logo / Title -->
            <div class="px-3 py-2 mb-2 -mt-2">
              <h1 class="text-lg font-bold text-white/90">MIDI Automation</h1>
              <!-- <p class="text-xs text-white/40">MIDI Automation</p> -->
              <p class="text-xs text-white/40">By YueLyn</p>
            </div>

            <!-- Navigation -->
            <nav class="flex flex-col gap-1">
              {#each navItems as item}
                <button
                  class="nav-item group flex items-center gap-3 px-3 py-2.5 rounded-lg text-left transition-all duration-200 {activeView ===
                  item.id
                    ? 'bg-white/10 text-white'
                    : 'text-white/60 hover:text-white hover:bg-white/5'}"
                  onclick={() => (activeView = item.id)}
                >
                  <div class="relative">
                    <Icon
                      icon={item.icon}
                      class="w-5 h-5 transition-transform duration-200 {activeView ===
                      item.id
                        ? 'scale-110'
                        : 'group-hover:scale-105'}"
                    />
                    {#if activeView === item.id}
                      <div
                        class="absolute -left-3 top-1/2 -translate-y-1/2 w-1 h-4 bg-[#1db954] rounded-r"
                        in:fly={{ x: -10, duration: 200 }}
                      ></div>
                    {/if}
                  </div>
                  <span class="font-medium text-sm">{item.label}</span>
                  {#if item.badge && item.badge() > 0}
                    <span
                      class="ml-auto text-xs px-2 py-0.5 rounded-full bg-white/10 text-white/60"
                      in:fade={{ duration: 150 }}
                    >
                      {item.badge()}
                    </span>
                  {/if}
                </button>
              {/each}
            </nav>

            <!-- Spacer -->
            <div class="flex-1"></div>

            <!-- Refresh Button -->
            <button
              class="flex items-center gap-2 px-3 py-2 rounded-lg text-white/60 hover:text-white hover:bg-white/5 transition-all w-full"
              onclick={loadMidiFiles}
              title="Refresh library"
            >
              <Icon icon="mdi:refresh" class="w-5 h-5" />
              <span class="font-medium text-sm">Refresh</span>
            </button>

            <!-- Keyboard Shortcuts Info -->
            <div class="px-3 py-3 bg-white/5 rounded-lg mt-2">
              <p
                class="text-xs font-semibold text-white/60 mb-2 flex items-center gap-2"
              >
                <Icon icon="mdi:keyboard" class="w-4 h-4" />
                Shortcuts
              </p>
              <div class="space-y-1">
                {#each shortcuts.slice(0, 3) as shortcut}
                  <div class="flex justify-between text-xs">
                    <span class="text-white/40">{shortcut.action}</span>
                    <span class="text-white/60 font-mono"
                      >{shortcut.key.split(" / ")[0]}</span
                    >
                  </div>
                {/each}
              </div>
            </div>
          </aside>

          <!-- Main Content -->
          <div class="flex-1 flex flex-col overflow-hidden spotify-main">
            <!-- Content Area with transitions -->
            <div
              class="flex-1 overflow-hidden px-6 py-4 {$isDraggable
                ? 'drag-handle'
                : ''} no-drag"
            >
              {#key activeView}
                <div
                  class="h-full"
                  in:fly={{ y: 10, duration: 200, delay: 50 }}
                  out:fade={{ duration: 100 }}
                >
                  {#if activeView === "library"}
                    <MidiFileList />
                  {:else if activeView === "queue"}
                    <PlaylistManager />
                  {:else if activeView === "favorites"}
                    <FavoritesView />
                  {:else if activeView === "playlists"}
                    <SavedPlaylistsView />
                  {/if}
                </div>
              {/key}
            </div>
          </div>
        </div>

        <!-- Bottom Player Bar -->
        <div
          class="spotify-player px-4 py-3 flex items-center justify-between gap-4 no-drag"
        >
          <!-- Now Playing -->
          <div class="flex items-center gap-4 w-64">
            <div
              class="w-12 h-12 rounded bg-white/5 flex items-center justify-center flex-shrink-0"
            >
              {#if $currentFile}
                <Icon icon="mdi:music-note" class="w-6 h-6 text-[#1db954]" />
              {:else}
                <Icon icon="mdi:music-note-off" class="w-6 h-6 text-white/30" />
              {/if}
            </div>
            <div class="min-w-0">
              <p class="text-sm font-semibold truncate text-white/90">
                {filename($currentFile)}
              </p>
              <p class="text-xs text-white/50 truncate">
                {#if $playlist.length > 0}
                  {$playlist.length} tracks in queue
                {:else}
                  No tracks in queue
                {/if}
              </p>
            </div>
          </div>

          <!-- Player Controls Center -->
          <div class="flex-1 max-w-xl">
            <PlaybackControls />
            <Timeline />
          </div>

          <!-- Right Controls -->
          <div class="flex items-center gap-2 w-64 justify-end">
            <button
              class="spotify-icon-button transition-all duration-200 {$loopMode
                ? 'text-[#1db954] bg-[#1db954]/10'
                : 'hover:text-white'}"
              onclick={toggleLoop}
              title={$loopMode ? "Loop enabled" : "Enable loop"}
            >
              <Icon icon="mdi:repeat" class="w-4 h-4" />
            </button>
            <button
              class="spotify-icon-button"
              onclick={() => (activeView = "queue")}
              title="View queue"
            >
              <Icon icon="mdi:playlist-play" class="w-4 h-4" />
            </button>
          </div>
        </div>
      {:else}
        <!-- Minimized view -->
        <div class="spotify-player p-4">
          <PlaybackControls compact={true} />
          <Timeline compact={true} />
        </div>
      {/if}
    </div>
  </main>
{/if}

<style>
  :global(body) {
    background: transparent;
  }

  .nav-item {
    position: relative;
    overflow: hidden;
  }

  .nav-item::before {
    content: "";
    position: absolute;
    inset: 0;
    background: linear-gradient(90deg, transparent, rgba(255, 255, 255, 0.03));
    opacity: 0;
    transition: opacity 0.2s;
  }

  .nav-item:hover::before {
    opacity: 1;
  }
</style>
