import { writable, get } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import Peer from 'peerjs';

// Band mode state
export const bandEnabled = writable(false);
export const isHost = writable(false);
export const roomCode = writable(null);
export const connectedPeers = writable([]); // [{ id, name, latency, trackId, slot, ready }]
export const myTrackId = writable(null);
export const mySlot = writable(null); // For split mode: which slot (0, 1, 2...) this player has
export const availableTracks = writable([]); // [{ id, name, noteCount }]
export const bandStatus = writable('disconnected'); // disconnected, connecting, connected, error
export const bandSongSelectMode = writable(false); // true when selecting song for band
export const bandSelectedSong = writable(null); // { name, path, ... }
export const bandPlayMode = writable('split'); // 'split' = auto-distribute notes, 'track' = each player picks a track
export const myReady = writable(false); // Member's ready state
export const bandFilePath = writable(null); // Path to use for playback (local or temp)
export const hostDelay = writable(300); // Host delay in ms (adjustable 0-500ms)

// Internal state
let peer = null;
let connections = new Map(); // peerId -> DataConnection
let latencyIntervals = new Map();
let syncInterval = null;

// Generate short room code
function generateRoomCode() {
  const chars = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789'; // No confusing chars (0,O,1,I)
  let code = '';
  for (let i = 0; i < 6; i++) {
    code += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return code;
}

// Create a room (host)
export async function createRoom(playerName = 'Host') {
  return new Promise((resolve, reject) => {
    const code = generateRoomCode();

    // PeerJS uses the room code as the peer ID for easy joining
    peer = new Peer(`wwm-${code}`, {
      debug: 1,
    });

    peer.on('open', (id) => {
      console.log('Room created:', code);
      isHost.set(true);
      roomCode.set(code);
      bandStatus.set('connected');

      // Add self to peers
      connectedPeers.set([{
        id: 'host',
        name: playerName,
        latency: 0,
        trackId: null,
        isHost: true,
        ready: true // Host is always ready
      }]);

      resolve(code);
    });

    peer.on('connection', (conn) => {
      handleIncomingConnection(conn);
    });

    peer.on('error', (err) => {
      console.error('Peer error:', err);
      bandStatus.set('error');
      reject(err);
    });

    peer.on('disconnected', () => {
      bandStatus.set('disconnected');
    });
  });
}

// Join a room (player)
export async function joinRoom(code, playerName = 'Player') {
  return new Promise((resolve, reject) => {
    code = code.toUpperCase().trim();

    let connectionTimeout = null;
    let connected = false;

    peer = new Peer({
      debug: 1,
    });

    const cleanup = () => {
      if (connectionTimeout) {
        clearTimeout(connectionTimeout);
        connectionTimeout = null;
      }
    };

    peer.on('open', () => {
      bandStatus.set('connecting');

      // Set connection timeout (10 seconds)
      connectionTimeout = setTimeout(() => {
        if (!connected) {
          console.error('Connection timeout - room may not exist');
          bandStatus.set('error');
          if (peer) {
            peer.destroy();
            peer = null;
          }
          reject(new Error('Room not found or connection timeout'));
        }
      }, 10000);

      // Connect to the host
      const conn = peer.connect(`wwm-${code}`, {
        metadata: { name: playerName }
      });

      conn.on('open', () => {
        connected = true;
        cleanup();

        console.log('Connected to room:', code);
        isHost.set(false);
        roomCode.set(code);
        bandStatus.set('connected');

        connections.set('host', conn);
        setupConnectionHandlers(conn, 'host');

        // Send join message
        conn.send({
          type: 'join',
          name: playerName
        });

        resolve(code);
      });

      conn.on('error', (err) => {
        cleanup();
        console.error('Connection error:', err);
        bandStatus.set('error');
        reject(err);
      });
    });

    peer.on('error', (err) => {
      cleanup();
      console.error('Peer error:', err);
      if (err.type === 'peer-unavailable') {
        bandStatus.set('error');
        reject(new Error('Room not found'));
      } else {
        bandStatus.set('error');
        reject(err);
      }
    });
  });
}

// Handle incoming connection (host only)
function handleIncomingConnection(conn) {
  const peerId = conn.peer;

  conn.on('open', () => {
    console.log('Player connected:', peerId);
    connections.set(peerId, conn);
    setupConnectionHandlers(conn, peerId);
  });
}

// Setup message handlers for a connection
function setupConnectionHandlers(conn, peerId) {
  conn.on('data', (data) => {
    handleMessage(data, peerId, conn);
  });

  conn.on('close', () => {
    console.log('Peer disconnected:', peerId);
    connections.delete(peerId);
    latencyIntervals.delete(peerId);

    connectedPeers.update(peers =>
      peers.filter(p => p.id !== peerId)
    );

    // Notify others if host
    if (get(isHost)) {
      broadcast({ type: 'peer_left', peerId });
    }
  });

  // Start latency measurement
  startLatencyMeasurement(peerId, conn);
}

// Handle incoming messages
function handleMessage(data, fromPeerId, conn) {
  const $isHost = get(isHost);

  switch (data.type) {
    case 'join':
      if ($isHost) {
        // Add new player
        const newPeer = {
          id: fromPeerId,
          name: data.name,
          latency: 0,
          trackId: null,
          isHost: false,
          ready: false
        };

        connectedPeers.update(peers => [...peers, newPeer]);

        // Send current state to new player
        const $bandSelectedSong = get(bandSelectedSong);
        conn.send({
          type: 'room_state',
          peers: get(connectedPeers),
          tracks: get(availableTracks),
          mode: get(bandPlayMode),
          song: $bandSelectedSong ? { name: $bandSelectedSong.name, filename: $bandSelectedSong.filename } : null
        });

        // If song is selected, send file data for the new player
        if ($bandSelectedSong && $bandSelectedSong.fileData) {
          conn.send({
            type: 'song_data',
            filename: $bandSelectedSong.filename,
            fileData: $bandSelectedSong.fileData
          });
        }

        // Notify others
        broadcast({ type: 'peer_joined', peer: newPeer }, fromPeerId);
      }
      break;

    case 'room_state':
      // Received room state from host
      connectedPeers.set(data.peers);
      availableTracks.set(data.tracks);
      if (data.mode) bandPlayMode.set(data.mode);
      break;

    case 'peer_joined':
      connectedPeers.update(peers => [...peers, data.peer]);
      break;

    case 'peer_left':
      connectedPeers.update(peers =>
        peers.filter(p => p.id !== data.peerId)
      );
      break;

    case 'ping':
      // Respond to latency ping
      conn.send({
        type: 'pong',
        timestamp: data.timestamp
      });
      break;

    case 'pong':
      // Calculate latency
      const latency = (Date.now() - data.timestamp) / 2;
      connectedPeers.update(peers =>
        peers.map(p => p.id === fromPeerId ? { ...p, latency: Math.round(latency) } : p)
      );
      break;

    case 'track_assign':
      // Host assigned a track
      connectedPeers.update(peers =>
        peers.map(p => p.id === data.peerId ? { ...p, trackId: data.trackId } : p)
      );
      if (data.peerId === peer.id || (data.peerId === 'host' && $isHost)) {
        myTrackId.set(data.trackId);
      }
      break;

    case 'tracks_update':
      availableTracks.set(data.tracks);
      break;

    case 'slot_assign':
      // Host assigned a slot for split mode
      connectedPeers.update(peers =>
        peers.map(p => p.id === data.peerId ? { ...p, slot: data.slot } : p)
      );
      if (data.peerId === peer.id || (data.peerId === 'host' && $isHost)) {
        mySlot.set(data.slot);
      }
      break;

    case 'mode_change':
      bandPlayMode.set(data.mode);
      break;

    case 'play':
      // Synchronized play command
      handlePlayCommand(data);
      break;

    case 'pause':
      handlePauseCommand(data);
      break;

    case 'stop':
      handleStopCommand();
      break;

    case 'seek':
      handleSeekCommand(data);
      break;

    case 'sync':
      // Sync pulse for drift correction
      handleSyncPulse(data);
      break;

    case 'room_closed':
      // Host left, all members must leave
      console.log('Host closed the room');
      handleRoomClosed();
      break;

    case 'ready':
      // Member toggled ready state
      if ($isHost) {
        connectedPeers.update(peers =>
          peers.map(p => p.id === fromPeerId ? { ...p, ready: data.ready } : p)
        );
        // Broadcast updated ready state to all
        broadcast({ type: 'ready_update', peerId: fromPeerId, ready: data.ready }, fromPeerId);
      }
      break;

    case 'ready_update':
      // Ready state broadcast from host
      connectedPeers.update(peers =>
        peers.map(p => p.id === data.peerId ? { ...p, ready: data.ready } : p)
      );
      break;

    case 'song_select':
      // Host selected a song - check if we have it locally
      handleSongSelect(data);
      break;

    case 'song_data':
      // Receive song file data from host
      handleSongData(data);
      break;

    case 'ready_reset':
      // Reset all ready states (on stop)
      myReady.set(false);
      connectedPeers.update(peers =>
        peers.map(p => ({ ...p, ready: p.isHost ? true : false }))
      );
      break;
  }
}

// Latency measurement
function startLatencyMeasurement(peerId, conn) {
  // Measure every 2 seconds
  const interval = setInterval(() => {
    if (conn.open) {
      conn.send({
        type: 'ping',
        timestamp: Date.now()
      });
    }
  }, 2000);

  latencyIntervals.set(peerId, interval);

  // Initial ping
  conn.send({ type: 'ping', timestamp: Date.now() });
}

// Broadcast message to all peers (host only)
function broadcast(data, excludePeerId = null) {
  connections.forEach((conn, peerId) => {
    if (peerId !== excludePeerId && conn.open) {
      conn.send(data);
    }
  });
}

// Send to host (player only)
function sendToHost(data) {
  const hostConn = connections.get('host');
  if (hostConn && hostConn.open) {
    hostConn.send(data);
  }
}

// Set available tracks (host, after loading MIDI)
export function setAvailableTracks(tracks) {
  availableTracks.set(tracks);

  if (get(isHost)) {
    broadcast({ type: 'tracks_update', tracks });
  }
}

// Load tracks from a MIDI file (host only)
export async function loadTracksFromFile(filePath) {
  if (!get(isHost)) return;

  try {
    const tracks = await invoke('get_midi_tracks', { path: filePath });
    setAvailableTracks(tracks);
    return tracks;
  } catch (error) {
    console.error('Failed to load tracks:', error);
    return [];
  }
}

// Start song selection mode
export function startBandSongSelect() {
  bandSongSelectMode.set(true);
}

// Select a song for band mode
export async function selectBandSong(file) {
  if (!get(isHost)) return;

  // Extract filename from path
  const filename = file.path.split(/[\\/]/).pop();

  // Read file as base64 for transfer
  let fileData = null;
  try {
    fileData = await invoke('read_midi_base64', { path: file.path });
  } catch (err) {
    console.error('Failed to read MIDI file:', err);
  }

  // Store with file data for new joiners
  bandSelectedSong.set({ ...file, filename, fileData });
  bandSongSelectMode.set(false);
  bandFilePath.set(file.path);

  // Reset all ready states
  connectedPeers.update(peers =>
    peers.map(p => ({ ...p, ready: p.isHost ? true : false }))
  );

  // Auto-load tracks
  if (file?.path) {
    await loadTracksFromFile(file.path);
  }

  // Broadcast song select to members
  broadcast({ type: 'song_select', name: file.name, filename });

  // Send file data to all members
  if (fileData) {
    broadcast({ type: 'song_data', filename, fileData });
  }
}

// Cancel song selection
export function cancelBandSongSelect() {
  bandSongSelectMode.set(false);
}

// Assign track to player (host only)
export function assignTrack(peerId, trackId) {
  if (!get(isHost)) return;

  connectedPeers.update(peers =>
    peers.map(p => p.id === peerId ? { ...p, trackId } : p)
  );

  if (peerId === 'host') {
    myTrackId.set(trackId);
  }

  broadcast({ type: 'track_assign', peerId, trackId });
}

// Assign slot to player for split mode (host only)
export function assignSlot(peerId, slot) {
  if (!get(isHost)) return;

  connectedPeers.update(peers =>
    peers.map(p => p.id === peerId ? { ...p, slot } : p)
  );

  if (peerId === 'host') {
    mySlot.set(slot);
  }

  broadcast({ type: 'slot_assign', peerId, slot });
}

// Auto-assign slots to all players (host only)
export function autoAssignSlots() {
  if (!get(isHost)) return;

  const peers = get(connectedPeers);
  peers.forEach((peer, index) => {
    assignSlot(peer.id, index);
  });
}

// Set band play mode (host only)
export function setBandPlayMode(mode) {
  if (!get(isHost)) return;

  bandPlayMode.set(mode);
  broadcast({ type: 'mode_change', mode });

  // Auto-assign slots when switching to split mode
  if (mode === 'split') {
    autoAssignSlots();
  }
}

// Synchronized play (host only)
export function bandPlay(position = 0) {
  if (!get(isHost)) return;

  const peers = get(connectedPeers);
  const maxLatency = Math.max(...peers.map(p => p.latency), 0);
  const mode = get(bandPlayMode);
  const totalPlayers = peers.length;

  // Schedule start time with buffer for highest latency peer
  const buffer = Math.max(maxLatency * 2 + 100, 300); // At least 300ms
  const startAt = Date.now() + buffer;

  // For split mode, each peer needs their slot info
  const playCmd = {
    type: 'play',
    startAt,
    position,
    mode,
    totalPlayers
  };

  broadcast(playCmd);
  handlePlayCommand(playCmd); // Also execute locally
}

// Synchronized pause (host only)
export function bandPause() {
  if (!get(isHost)) return;

  const pauseCmd = { type: 'pause', timestamp: Date.now() };
  broadcast(pauseCmd);
  handlePauseCommand(pauseCmd);
}

// Synchronized stop (host only)
export function bandStop() {
  if (!get(isHost)) return;

  broadcast({ type: 'stop' });
  handleStopCommand();

  // Reset all ready states
  broadcast({ type: 'ready_reset' });
  connectedPeers.update(peers =>
    peers.map(p => ({ ...p, ready: p.isHost ? true : false }))
  );
}

// Toggle ready state (member only)
export function toggleReady() {
  if (get(isHost)) return; // Host is always ready

  const newReady = !get(myReady);
  myReady.set(newReady);

  // Send to host
  const hostConn = connections.get('host');
  if (hostConn && hostConn.open) {
    hostConn.send({ type: 'ready', ready: newReady });
  }
}

// Check if all members are ready (for host)
export function allMembersReady() {
  const peers = get(connectedPeers);
  // All non-host peers must be ready
  return peers.filter(p => !p.isHost).every(p => p.ready);
}

// Synchronized seek (host only)
export function bandSeek(position) {
  if (!get(isHost)) return;

  const peers = get(connectedPeers);
  const maxLatency = Math.max(...peers.map(p => p.latency), 0);

  // Schedule seek time with buffer for highest latency peer
  const buffer = Math.max(maxLatency * 2 + 50, 150); // At least 150ms
  const seekAt = Date.now() + buffer;

  const seekCmd = {
    type: 'seek',
    seekAt,
    position
  };

  broadcast(seekCmd);
  handleSeekCommand(seekCmd); // Also execute locally
}

// Handle play command (all peers)
async function handlePlayCommand(data) {
  const { startAt, position, mode, totalPlayers } = data;
  const now = Date.now();
  const $isHost = get(isHost);

  // Host adds extra delay to let members receive and process the command
  const hostOffset = $isHost ? get(hostDelay) : 0;
  const delay = (startAt - now) + hostOffset;

  // Get this player's slot
  const $mySlot = get(mySlot) ?? 0;
  const $bandSelectedSong = get(bandSelectedSong);
  const $myTrackId = get(myTrackId);

  console.log(`Play scheduled in ${delay}ms (host offset: ${hostOffset}ms) at position ${position}, mode: ${mode}, slot: ${$mySlot}/${totalPlayers}`);

  // Import player functions
  const { playMidiBand, currentFile, seekTo } = await import('./player.js');

  // Use band selected song or current file
  const fileToPlay = $bandSelectedSong || get(currentFile);

  const playOptions = {
    mode: mode || 'split',
    slot: $mySlot,
    totalPlayers: totalPlayers || 1,
    trackId: $myTrackId
  };

  if (delay > 0) {
    setTimeout(async () => {
      if (fileToPlay) {
        await seekTo(position);
        await playMidiBand(fileToPlay, playOptions);
      }
    }, delay);
  } else {
    // Already past start time, play immediately
    if (fileToPlay) {
      await seekTo(position);
      await playMidiBand(fileToPlay, playOptions);
    }
  }
}

// Handle pause command
async function handlePauseCommand(data) {
  const { pauseResume, isPaused } = await import('./player.js');
  const $isPaused = get(isPaused);

  if (!$isPaused) {
    await pauseResume();
  }
}

// Handle stop command
async function handleStopCommand() {
  const { stopPlayback } = await import('./player.js');
  await stopPlayback();

  // Reset ready state for all (local only, host broadcasts separately)
  myReady.set(false);
}

// Handle song select from host (member side)
async function handleSongSelect(data) {
  const { filename, name } = data;

  // Reset ready state when song changes
  myReady.set(false);

  // Check if we have this file locally
  const localPath = await invoke('check_midi_exists', { filename });

  if (localPath) {
    // We have the file locally
    console.log('Using local file:', localPath);
    bandFilePath.set(localPath);
    bandSelectedSong.set({ name, filename, path: localPath });
  } else {
    // We don't have the file - wait for song_data
    console.log('File not found locally, waiting for transfer:', filename);
    bandFilePath.set(null);
    bandSelectedSong.set({ name, filename, path: null, pending: true });
  }
}

// Handle song data transfer from host (member side)
async function handleSongData(data) {
  const { filename, fileData } = data;

  try {
    // Save to temp
    const tempPath = await invoke('save_temp_midi', { filename, dataBase64: fileData });
    console.log('Saved temp file:', tempPath);

    // Update state
    bandFilePath.set(tempPath);
    bandSelectedSong.update(song => song ? { ...song, path: tempPath, pending: false } : null);
  } catch (err) {
    console.error('Failed to save temp MIDI:', err);
  }
}

// Handle seek command (all peers)
async function handleSeekCommand(data) {
  const { seekAt, position } = data;
  const now = Date.now();
  const delay = seekAt - now;

  const { seekTo } = await import('./player.js');

  if (delay > 0) {
    // Wait until scheduled time for sync
    setTimeout(async () => {
      await seekTo(position);
    }, delay);
  } else {
    // Already past seek time, execute immediately
    await seekTo(position);
  }
}

// Handle sync pulse for drift correction
async function handleSyncPulse(data) {
  // TODO: Implement drift correction
  // Compare data.position with our current position
  // Nudge playback if difference > threshold
}

// Start sync pulse (host only, call during playback)
export function startSyncPulse() {
  if (!get(isHost)) return;

  // Send sync every 5 seconds
  syncInterval = setInterval(async () => {
    const { currentPosition } = await import('./player.js');
    const position = get(currentPosition);

    broadcast({
      type: 'sync',
      position,
      timestamp: Date.now()
    });
  }, 5000);
}

export function stopSyncPulse() {
  if (syncInterval) {
    clearInterval(syncInterval);
    syncInterval = null;
  }
}

// Handle room closed by host (for members)
async function handleRoomClosed() {
  // Stop any playback
  const { stopPlayback } = await import('./player.js');
  await stopPlayback();

  // Clean up without broadcasting (we're receiving, not sending)
  stopSyncPulse();

  latencyIntervals.forEach(interval => clearInterval(interval));
  latencyIntervals.clear();

  connections.forEach(conn => conn.close());
  connections.clear();

  if (peer) {
    peer.destroy();
    peer = null;
  }

  // Reset state
  bandEnabled.set(false);
  isHost.set(false);
  roomCode.set(null);
  connectedPeers.set([]);
  myTrackId.set(null);
  mySlot.set(null);
  availableTracks.set([]);
  bandStatus.set('disconnected');
  bandPlayMode.set('split');
  bandSelectedSong.set(null);
}

// Leave room / cleanup
export function leaveRoom() {
  const $isHost = get(isHost);

  // If host is leaving, notify all members first
  if ($isHost) {
    broadcast({ type: 'room_closed' });
  }

  stopSyncPulse();

  latencyIntervals.forEach(interval => clearInterval(interval));
  latencyIntervals.clear();

  connections.forEach(conn => conn.close());
  connections.clear();

  if (peer) {
    peer.destroy();
    peer = null;
  }

  // Reset state
  bandEnabled.set(false);
  isHost.set(false);
  roomCode.set(null);
  connectedPeers.set([]);
  myTrackId.set(null);
  mySlot.set(null);
  availableTracks.set([]);
  bandStatus.set('disconnected');
  bandPlayMode.set('split');
  bandSelectedSong.set(null);
}

// Toggle band mode
export function toggleBandMode() {
  const enabled = !get(bandEnabled);
  bandEnabled.set(enabled);

  if (!enabled) {
    leaveRoom();
  }
}
