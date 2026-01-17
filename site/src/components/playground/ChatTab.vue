<script setup>
import { ref, computed, watch, onMounted, onUnmounted, nextTick } from 'vue'
import { useClasp } from '../../composables/useClasp'

const { connected, sessionId, subscribe, emit, set, settings } = useClasp()

const room = ref('lobby')
const nickname = ref('')
const message = ref('')
const messages = ref([])
const participants = ref(new Map())
const messagesContainer = ref(null)
const inputRef = ref(null)
const isTyping = ref(false)
const typingUsers = ref(new Map())
const showRoomList = ref(false)
const inRoom = ref(false)

const popularRooms = ['lobby', 'general', 'tech', 'creative', 'random']

let unsubMessages = null
let unsubPresence = null
let unsubTyping = null
let presenceInterval = null
let typingTimeout = null

const sortedParticipants = computed(() => {
  return Array.from(participants.value.entries())
    .map(([id, data]) => ({ id, ...data }))
    .sort((a, b) => a.name.localeCompare(b.name))
})

const typingList = computed(() => {
  return Array.from(typingUsers.value.entries())
    .filter(([id]) => id !== sessionId.value)
    .map(([, data]) => data.name)
})

function joinRoom(roomName = null) {
  if (!connected.value || !nickname.value.trim()) return

  if (roomName) {
    room.value = roomName
  }
  showRoomList.value = false

  // Add system message
  addSystemMessage(`Joining #${room.value}...`)

  // Subscribe to messages
  const msgPattern = `/chat/${room.value}/messages`
  unsubMessages = subscribe(msgPattern, (payload, address) => {
    if (payload && typeof payload === 'object') {
      // Skip messages from ourselves (we already added them locally)
      if (payload.fromId === sessionId.value) return

      messages.value.push({
        id: Date.now() + Math.random(),
        type: 'message',
        ...payload,
        received: new Date().toLocaleTimeString(),
      })
      scrollToBottom()
    }
  })

  // Subscribe to presence
  const presencePattern = `/chat/${room.value}/presence/*`
  unsubPresence = subscribe(presencePattern, (data, address) => {
    const userId = address.split('/').pop()
    if (data === null) {
      const user = participants.value.get(userId)
      if (user && userId !== sessionId.value) {
        addSystemMessage(`${user.name} left the room`)
      }
      participants.value.delete(userId)
    } else {
      const isNew = !participants.value.has(userId)
      participants.value.set(userId, data)
      if (isNew && userId !== sessionId.value) {
        addSystemMessage(`${data.name} joined the room`)
      }
    }
  })

  // Subscribe to typing indicators
  const typingPattern = `/chat/${room.value}/typing/*`
  unsubTyping = subscribe(typingPattern, (data, address) => {
    const userId = address.split('/').pop()
    if (data === null || data === false) {
      typingUsers.value.delete(userId)
    } else {
      typingUsers.value.set(userId, data)
      // Auto-clear typing after 3 seconds
      setTimeout(() => {
        typingUsers.value.delete(userId)
      }, 3000)
    }
  })

  // Announce our presence
  announcePresence()
  addSystemMessage(`Connected to #${room.value}`)

  // Keep announcing presence periodically
  presenceInterval = setInterval(announcePresence, 10000)

  // Mark as in room
  inRoom.value = true

  // Focus input
  nextTick(() => inputRef.value?.focus())
}

function addSystemMessage(text) {
  messages.value.push({
    id: Date.now() + Math.random(),
    type: 'system',
    text,
    timestamp: Date.now(),
  })
  scrollToBottom()
}

function announcePresence() {
  if (!connected.value || !sessionId.value) return
  set(`/chat/${room.value}/presence/${sessionId.value}`, {
    name: nickname.value,
    joinedAt: Date.now(),
  })
}

function leaveRoom() {
  if (unsubMessages) {
    unsubMessages()
    unsubMessages = null
  }
  if (unsubPresence) {
    unsubPresence()
    unsubPresence = null
  }
  if (unsubTyping) {
    unsubTyping()
    unsubTyping = null
  }
  if (presenceInterval) {
    clearInterval(presenceInterval)
    presenceInterval = null
  }

  // Clear presence and typing
  if (connected.value && sessionId.value) {
    set(`/chat/${room.value}/presence/${sessionId.value}`, null)
    set(`/chat/${room.value}/typing/${sessionId.value}`, null)
  }

  messages.value = []
  participants.value.clear()
  typingUsers.value.clear()
  inRoom.value = false
}

function sendMessage() {
  if (!connected.value || !message.value.trim()) return

  const msgData = {
    from: nickname.value,
    fromId: sessionId.value,
    text: message.value,
    timestamp: Date.now(),
  }

  // Add message to local list immediately so user sees their own message
  messages.value.push({
    id: Date.now() + Math.random(),
    type: 'message',
    ...msgData,
    received: new Date().toLocaleTimeString(),
  })
  scrollToBottom()

  // Emit to server for other participants
  emit(`/chat/${room.value}/messages`, msgData)

  // Clear typing indicator
  set(`/chat/${room.value}/typing/${sessionId.value}`, null)
  isTyping.value = false

  message.value = ''
  nextTick(() => inputRef.value?.focus())
}

function handleTyping() {
  if (!connected.value || !sessionId.value) return

  // Set typing indicator
  if (!isTyping.value) {
    isTyping.value = true
    set(`/chat/${room.value}/typing/${sessionId.value}`, {
      name: nickname.value,
      timestamp: Date.now(),
    })
  }

  // Clear typing after 2 seconds of inactivity
  clearTimeout(typingTimeout)
  typingTimeout = setTimeout(() => {
    isTyping.value = false
    set(`/chat/${room.value}/typing/${sessionId.value}`, null)
  }, 2000)
}

function scrollToBottom() {
  nextTick(() => {
    if (messagesContainer.value) {
      messagesContainer.value.scrollTop = messagesContainer.value.scrollHeight
    }
  })
}

function formatTime(timestamp) {
  return new Date(timestamp).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })
}

function getInitials(name) {
  return name.split(' ').map(n => n[0]).join('').toUpperCase().slice(0, 2)
}

function getAvatarColor(name) {
  const colors = ['#FF5F1F', '#2196F3', '#4CAF50', '#9C27B0', '#FF9800', '#00BCD4', '#E91E63', '#607D8B']
  let hash = 0
  for (let i = 0; i < name.length; i++) {
    hash = name.charCodeAt(i) + ((hash << 5) - hash)
  }
  return colors[Math.abs(hash) % colors.length]
}

// Set default nickname
onMounted(() => {
  nickname.value = settings.name || 'Anonymous'
})

// Cleanup on unmount
onUnmounted(() => {
  leaveRoom()
})

// Leave room on disconnect
watch(connected, (isConnected) => {
  if (!isConnected) {
    leaveRoom()
  }
})
</script>

<template>
  <div class="chat-tab">
    <!-- Join Screen -->
    <div v-if="!inRoom" class="join-screen">
      <div class="join-card">
        <div class="join-header">
          <svg class="chat-icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
            <path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"/>
          </svg>
          <h2>CLASP Chat</h2>
          <p>Real-time cross-device messaging powered by CLASP events</p>
        </div>

        <div class="join-form">
          <div class="field">
            <label>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/>
                <circle cx="12" cy="7" r="4"/>
              </svg>
              Nickname
            </label>
            <input
              v-model="nickname"
              type="text"
              placeholder="Enter your name"
              :disabled="!connected"
              @keyup.enter="joinRoom()"
            />
          </div>

          <div class="field">
            <label>
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                <path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"/>
                <polyline points="22,6 12,13 2,6"/>
              </svg>
              Room
            </label>
            <div class="room-input-wrapper">
              <span class="room-prefix">#</span>
              <input
                v-model="room"
                type="text"
                placeholder="lobby"
                :disabled="!connected"
                @keyup.enter="joinRoom()"
              />
            </div>
          </div>

          <div class="popular-rooms">
            <span class="rooms-label">Popular rooms:</span>
            <button
              v-for="r in popularRooms"
              :key="r"
              :class="['room-chip', { active: room === r }]"
              @click="room = r"
              :disabled="!connected"
            >
              #{{ r }}
            </button>
          </div>

          <button
            class="join-btn"
            @click="joinRoom()"
            :disabled="!connected || !nickname.trim()"
          >
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <path d="M15 3h4a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2h-4"/>
              <polyline points="10 17 15 12 10 7"/>
              <line x1="15" y1="12" x2="3" y2="12"/>
            </svg>
            Join Room
          </button>

          <p v-if="!connected" class="connect-hint">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <circle cx="12" cy="12" r="10"/>
              <line x1="12" y1="8" x2="12" y2="12"/>
              <line x1="12" y1="16" x2="12.01" y2="16"/>
            </svg>
            Connect to a CLASP server first
          </p>
        </div>
      </div>
    </div>

    <!-- Chat Room -->
    <div v-else class="chat-room">
      <!-- Room Header -->
      <div class="room-header">
        <div class="room-info">
          <button class="back-btn" @click="leaveRoom">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
              <polyline points="15 18 9 12 15 6"/>
            </svg>
          </button>
          <div class="room-details">
            <h3>#{{ room }}</h3>
            <span class="member-count">{{ sortedParticipants.length }} online</span>
          </div>
        </div>
        <div class="room-actions">
          <button class="header-btn" @click="showRoomList = !showRoomList" title="Switch room">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <rect x="3" y="3" width="7" height="7"/>
              <rect x="14" y="3" width="7" height="7"/>
              <rect x="14" y="14" width="7" height="7"/>
              <rect x="3" y="14" width="7" height="7"/>
            </svg>
          </button>
        </div>
      </div>

      <!-- Room Switcher Dropdown -->
      <div v-if="showRoomList" class="room-switcher">
        <div class="switcher-header">Switch Room</div>
        <button
          v-for="r in popularRooms"
          :key="r"
          :class="['switcher-item', { active: room === r }]"
          @click="leaveRoom(); joinRoom(r)"
        >
          <span class="room-hash">#</span>
          {{ r }}
          <span v-if="room === r" class="current-badge">current</span>
        </button>
      </div>

      <!-- Chat Layout -->
      <div class="chat-layout">
        <!-- Messages Area -->
        <div class="messages-area">
          <div class="messages-scroll" ref="messagesContainer">
            <div v-if="!messages.length" class="empty-state">
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1">
                <path d="M21 11.5a8.38 8.38 0 0 1-.9 3.8 8.5 8.5 0 0 1-7.6 4.7 8.38 8.38 0 0 1-3.8-.9L3 21l1.9-5.7a8.38 8.38 0 0 1-.9-3.8 8.5 8.5 0 0 1 4.7-7.6 8.38 8.38 0 0 1 3.8-.9h.5a8.48 8.48 0 0 1 8 8v.5z"/>
              </svg>
              <p>No messages yet</p>
              <span>Be the first to say hello!</span>
            </div>

            <template v-for="(msg, idx) in messages" :key="msg.id">
              <!-- System Message -->
              <div v-if="msg.type === 'system'" class="system-message">
                <span class="system-dot"></span>
                {{ msg.text }}
              </div>

              <!-- Chat Message -->
              <div
                v-else
                :class="['message', { own: msg.fromId === sessionId, grouped: idx > 0 && messages[idx-1].fromId === msg.fromId && messages[idx-1].type !== 'system' }]"
              >
                <div v-if="!(idx > 0 && messages[idx-1].fromId === msg.fromId && messages[idx-1].type !== 'system')" class="avatar" :style="{ background: getAvatarColor(msg.from) }">
                  {{ getInitials(msg.from) }}
                </div>
                <div class="message-content">
                  <div v-if="!(idx > 0 && messages[idx-1].fromId === msg.fromId && messages[idx-1].type !== 'system')" class="message-meta">
                    <span class="sender-name">{{ msg.from }}</span>
                    <span class="message-time">{{ formatTime(msg.timestamp) }}</span>
                  </div>
                  <div class="message-bubble">{{ msg.text }}</div>
                </div>
              </div>
            </template>

            <!-- Typing Indicator -->
            <div v-if="typingList.length" class="typing-indicator">
              <div class="typing-dots">
                <span></span><span></span><span></span>
              </div>
              <span class="typing-text">
                {{ typingList.join(', ') }} {{ typingList.length === 1 ? 'is' : 'are' }} typing...
              </span>
            </div>
          </div>

          <!-- Message Input -->
          <div class="message-composer">
            <input
              ref="inputRef"
              v-model="message"
              type="text"
              placeholder="Type a message..."
              @keyup.enter="sendMessage"
              @input="handleTyping"
              :disabled="!connected"
            />
            <button
              class="send-btn"
              @click="sendMessage"
              :disabled="!connected || !message.trim()"
            >
              <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2">
                <line x1="22" y1="2" x2="11" y2="13"/>
                <polygon points="22 2 15 22 11 13 2 9 22 2"/>
              </svg>
            </button>
          </div>
        </div>

        <!-- Participants Sidebar -->
        <div class="participants-sidebar">
          <div class="sidebar-header">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
              <path d="M17 21v-2a4 4 0 0 0-4-4H5a4 4 0 0 0-4 4v2"/>
              <circle cx="9" cy="7" r="4"/>
              <path d="M23 21v-2a4 4 0 0 0-3-3.87"/>
              <path d="M16 3.13a4 4 0 0 1 0 7.75"/>
            </svg>
            Participants
          </div>
          <div class="participants-list">
            <div
              v-for="p in sortedParticipants"
              :key="p.id"
              :class="['participant', { self: p.id === sessionId }]"
            >
              <div class="participant-avatar" :style="{ background: getAvatarColor(p.name) }">
                {{ getInitials(p.name) }}
              </div>
              <span class="participant-name">{{ p.name }}</span>
              <span v-if="p.id === sessionId" class="you-tag">you</span>
              <span class="online-indicator"></span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<style scoped>
.chat-tab {
  height: 100%;
  display: flex;
  flex-direction: column;
}

/* Join Screen */
.join-screen {
  flex: 1;
  display: flex;
  align-items: center;
  justify-content: center;
  padding: 2rem;
}

.join-card {
  width: 100%;
  max-width: 420px;
  background: rgba(255,255,255,0.6);
  border: 1px solid rgba(0,0,0,0.1);
  padding: 2.5rem;
}

.join-header {
  text-align: center;
  margin-bottom: 2rem;
}

.join-header .chat-icon {
  width: 48px;
  height: 48px;
  margin-bottom: 1rem;
  opacity: 0.3;
}

.join-header h2 {
  margin: 0 0 0.5rem;
  font-size: 1.4rem;
  letter-spacing: 0.15em;
  font-weight: 500;
}

.join-header p {
  margin: 0;
  font-size: 0.85rem;
  opacity: 0.6;
  line-height: 1.5;
}

.join-form {
  display: flex;
  flex-direction: column;
  gap: 1.25rem;
}

.field {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.field label {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  font-size: 0.75rem;
  letter-spacing: 0.1em;
  text-transform: uppercase;
  opacity: 0.6;
}

.field label svg {
  width: 14px;
  height: 14px;
}

.field input {
  padding: 0.8rem 1rem;
  border: 1px solid rgba(0,0,0,0.12);
  background: rgba(255,255,255,0.8);
  font-family: inherit;
  font-size: 0.95rem;
  transition: border-color 0.15s;
}

.field input:focus {
  outline: none;
  border-color: var(--accent);
}

.field input:disabled {
  opacity: 0.5;
}

.room-input-wrapper {
  display: flex;
  align-items: center;
  border: 1px solid rgba(0,0,0,0.12);
  background: rgba(255,255,255,0.8);
}

.room-input-wrapper:focus-within {
  border-color: var(--accent);
}

.room-prefix {
  padding: 0.8rem 0 0.8rem 1rem;
  opacity: 0.4;
  font-size: 0.95rem;
}

.room-input-wrapper input {
  border: none;
  padding-left: 0.25rem;
  flex: 1;
}

.popular-rooms {
  display: flex;
  align-items: center;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.rooms-label {
  font-size: 0.7rem;
  opacity: 0.5;
  letter-spacing: 0.05em;
}

.room-chip {
  padding: 0.35rem 0.6rem;
  font-size: 0.75rem;
  border: 1px solid rgba(0,0,0,0.12);
  background: transparent;
  cursor: pointer;
  font-family: inherit;
  transition: all 0.15s;
}

.room-chip:hover:not(:disabled) {
  background: rgba(0,0,0,0.05);
}

.room-chip.active {
  background: var(--ink);
  color: var(--paper);
  border-color: var(--ink);
}

.room-chip:disabled {
  opacity: 0.4;
}

.join-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.6rem;
  padding: 1rem;
  background: var(--ink);
  color: var(--paper);
  border: none;
  font-family: inherit;
  font-size: 0.9rem;
  letter-spacing: 0.12em;
  cursor: pointer;
  transition: background 0.15s;
  margin-top: 0.5rem;
}

.join-btn svg {
  width: 18px;
  height: 18px;
}

.join-btn:hover:not(:disabled) {
  background: var(--accent);
}

.join-btn:disabled {
  opacity: 0.4;
  cursor: not-allowed;
}

.connect-hint {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  margin: 0;
  font-size: 0.8rem;
  opacity: 0.5;
  text-align: center;
}

.connect-hint svg {
  width: 16px;
  height: 16px;
}

/* Chat Room */
.chat-room {
  flex: 1;
  display: flex;
  flex-direction: column;
  background: rgba(255,255,255,0.3);
  border: 1px solid rgba(0,0,0,0.1);
  min-height: 0;
}

.room-header {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.75rem 1rem;
  background: rgba(255,255,255,0.5);
  border-bottom: 1px solid rgba(0,0,0,0.08);
}

.room-info {
  display: flex;
  align-items: center;
  gap: 0.75rem;
}

.back-btn {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: 1px solid rgba(0,0,0,0.1);
  cursor: pointer;
  transition: all 0.15s;
}

.back-btn:hover {
  background: rgba(0,0,0,0.05);
  border-color: rgba(0,0,0,0.2);
}

.back-btn svg {
  width: 16px;
  height: 16px;
}

.room-details h3 {
  margin: 0;
  font-size: 1rem;
  font-weight: 600;
  letter-spacing: 0.05em;
}

.member-count {
  font-size: 0.7rem;
  opacity: 0.5;
}

.room-actions {
  display: flex;
  gap: 0.5rem;
}

.header-btn {
  width: 32px;
  height: 32px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: transparent;
  border: 1px solid rgba(0,0,0,0.1);
  cursor: pointer;
  transition: all 0.15s;
}

.header-btn:hover {
  background: rgba(0,0,0,0.05);
}

.header-btn svg {
  width: 16px;
  height: 16px;
  opacity: 0.6;
}

/* Room Switcher */
.room-switcher {
  position: absolute;
  right: 1rem;
  top: 50px;
  z-index: 100;
  background: white;
  border: 1px solid rgba(0,0,0,0.15);
  box-shadow: 0 4px 12px rgba(0,0,0,0.1);
  min-width: 160px;
}

.switcher-header {
  padding: 0.6rem 0.8rem;
  font-size: 0.65rem;
  letter-spacing: 0.15em;
  text-transform: uppercase;
  opacity: 0.5;
  border-bottom: 1px solid rgba(0,0,0,0.08);
}

.switcher-item {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  width: 100%;
  padding: 0.6rem 0.8rem;
  background: none;
  border: none;
  font-family: inherit;
  font-size: 0.85rem;
  cursor: pointer;
  text-align: left;
  transition: background 0.15s;
}

.switcher-item:hover {
  background: rgba(0,0,0,0.05);
}

.switcher-item.active {
  background: rgba(255, 95, 31, 0.08);
}

.room-hash {
  opacity: 0.4;
}

.current-badge {
  margin-left: auto;
  font-size: 0.65rem;
  opacity: 0.4;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

/* Chat Layout */
.chat-layout {
  flex: 1;
  display: grid;
  grid-template-columns: 1fr 200px;
  min-height: 0;
}

.messages-area {
  display: flex;
  flex-direction: column;
  min-height: 0;
  border-right: 1px solid rgba(0,0,0,0.08);
}

.messages-scroll {
  flex: 1;
  overflow-y: auto;
  padding: 1rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.empty-state {
  flex: 1;
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  text-align: center;
  opacity: 0.4;
}

.empty-state svg {
  width: 48px;
  height: 48px;
  margin-bottom: 1rem;
}

.empty-state p {
  margin: 0 0 0.25rem;
  font-size: 0.9rem;
}

.empty-state span {
  font-size: 0.8rem;
}

/* System Message */
.system-message {
  display: flex;
  align-items: center;
  justify-content: center;
  gap: 0.5rem;
  padding: 0.5rem;
  font-size: 0.75rem;
  opacity: 0.5;
}

.system-dot {
  width: 4px;
  height: 4px;
  background: currentColor;
  border-radius: 50%;
}

/* Chat Message */
.message {
  display: flex;
  gap: 0.75rem;
  padding: 0.25rem 0;
}

.message.grouped {
  padding-left: 44px;
}

.message.own {
  flex-direction: row-reverse;
}

.message.own.grouped {
  padding-left: 0;
  padding-right: 44px;
}

.avatar {
  width: 36px;
  height: 36px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  font-size: 0.7rem;
  font-weight: 600;
  flex-shrink: 0;
}

.message-content {
  max-width: 70%;
  display: flex;
  flex-direction: column;
  gap: 0.25rem;
}

.message.own .message-content {
  align-items: flex-end;
}

.message-meta {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.message.own .message-meta {
  flex-direction: row-reverse;
}

.sender-name {
  font-size: 0.75rem;
  font-weight: 600;
  letter-spacing: 0.02em;
}

.message-time {
  font-size: 0.65rem;
  opacity: 0.4;
}

.message-bubble {
  padding: 0.6rem 0.9rem;
  background: rgba(0,0,0,0.05);
  border-radius: 12px;
  border-top-left-radius: 4px;
  font-size: 0.9rem;
  line-height: 1.4;
  word-break: break-word;
}

.message.own .message-bubble {
  background: var(--accent);
  color: white;
  border-radius: 12px;
  border-top-right-radius: 4px;
}

.message.grouped .message-bubble {
  border-top-left-radius: 12px;
}

.message.own.grouped .message-bubble {
  border-top-right-radius: 12px;
}

/* Typing Indicator */
.typing-indicator {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0;
}

.typing-dots {
  display: flex;
  gap: 3px;
}

.typing-dots span {
  width: 6px;
  height: 6px;
  background: rgba(0,0,0,0.3);
  border-radius: 50%;
  animation: typing 1.4s infinite;
}

.typing-dots span:nth-child(2) { animation-delay: 0.2s; }
.typing-dots span:nth-child(3) { animation-delay: 0.4s; }

@keyframes typing {
  0%, 60%, 100% { transform: translateY(0); }
  30% { transform: translateY(-4px); }
}

.typing-text {
  font-size: 0.75rem;
  opacity: 0.5;
  font-style: italic;
}

/* Message Composer */
.message-composer {
  display: flex;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  background: rgba(255,255,255,0.5);
  border-top: 1px solid rgba(0,0,0,0.08);
}

.message-composer input {
  flex: 1;
  padding: 0.75rem 1rem;
  border: 1px solid rgba(0,0,0,0.1);
  background: white;
  font-family: inherit;
  font-size: 0.9rem;
  border-radius: 20px;
}

.message-composer input:focus {
  outline: none;
  border-color: var(--accent);
}

.send-btn {
  width: 42px;
  height: 42px;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--ink);
  border: none;
  border-radius: 50%;
  cursor: pointer;
  transition: background 0.15s;
}

.send-btn svg {
  width: 18px;
  height: 18px;
  color: white;
}

.send-btn:hover:not(:disabled) {
  background: var(--accent);
}

.send-btn:disabled {
  opacity: 0.3;
  cursor: not-allowed;
}

/* Participants Sidebar */
.participants-sidebar {
  display: flex;
  flex-direction: column;
  background: rgba(255,255,255,0.3);
}

.sidebar-header {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  font-size: 0.7rem;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  opacity: 0.5;
  border-bottom: 1px solid rgba(0,0,0,0.08);
}

.sidebar-header svg {
  width: 14px;
  height: 14px;
}

.participants-list {
  flex: 1;
  overflow-y: auto;
  padding: 0.75rem;
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.participant {
  display: flex;
  align-items: center;
  gap: 0.6rem;
  padding: 0.4rem;
  border-radius: 4px;
  transition: background 0.15s;
}

.participant:hover {
  background: rgba(0,0,0,0.03);
}

.participant.self {
  background: rgba(255, 95, 31, 0.05);
}

.participant-avatar {
  width: 28px;
  height: 28px;
  border-radius: 50%;
  display: flex;
  align-items: center;
  justify-content: center;
  color: white;
  font-size: 0.6rem;
  font-weight: 600;
  flex-shrink: 0;
}

.participant-name {
  flex: 1;
  font-size: 0.8rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.you-tag {
  font-size: 0.6rem;
  opacity: 0.4;
  text-transform: uppercase;
  letter-spacing: 0.05em;
}

.online-indicator {
  width: 8px;
  height: 8px;
  background: #4CAF50;
  border-radius: 50%;
  flex-shrink: 0;
}

/* Responsive */
@media (max-width: 768px) {
  .chat-layout {
    grid-template-columns: 1fr;
  }

  .participants-sidebar {
    display: none;
  }

  .message-content {
    max-width: 85%;
  }
}
</style>
