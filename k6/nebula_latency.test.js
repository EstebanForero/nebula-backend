// nebula_latency.test.js
import http from 'k6/http';
import ws from 'k6/ws';
import { check, sleep } from 'k6';
import { Trend } from 'k6/metrics';
import { htmlReport } from 'https://raw.githubusercontent.com/benc-uk/k6-reporter/3.0.1/dist/bundle.js';

// ========================
// CONFIG
// ========================

const API_BASE_URL = __ENV.API_BASE_URL || 'http://localhost:3838';

// ========================
// CUSTOM METRICS
// ========================

// HTTP endpoint latencies
export const http_me_latency = new Trend('http_me_latency', true);
export const http_rooms_latency = new Trend('http_rooms_latency', true);
export const http_messages_get_latency = new Trend('http_messages_get_latency', true);
export const http_messages_post_latency = new Trend('http_messages_post_latency', true);
export const http_room_join_latency = new Trend('http_room_join_latency', true);
export const http_room_leave_latency = new Trend('http_room_leave_latency', true);

// End-to-end message delivery latency: POST -> WS receive (receiver only)
export const message_delivery_latency = new Trend('message_delivery_latency', true);

// ========================
// SCENARIOS
// ========================

export const options = {
  scenarios: {
    // Basic HTTP latencies for core endpoints
    http_endpoint_latency: {
      executor: 'per-vu-iterations',
      exec: 'httpEndpointLatencyScenario',
      vus: 1,
      iterations: 10,
    },

    // Single receiver: 1 user connected via WebSocket,
    // measuring latency when sender posts messages.
    ws_single_receiver: {
      executor: 'constant-vus',
      exec: 'wsReceiverSingleScenario',
      vus: 1,
      duration: '20s',
      startTime: '5s',
    },

    // Many receivers: 100 users connected via WebSocket,
    // measuring latency under fan-out load.
    ws_many_receivers: {
      executor: 'constant-vus',
      exec: 'wsReceiversManyScenario',
      vus: 100,
      duration: '20s',
      startTime: '5s',
    },

    // Sender: a single user that only POSTS messages into the room.
    // Sender does not have a WS connection (you said sender does not receive its own messages).
    ws_sender: {
      executor: 'constant-vus',
      exec: 'wsSenderScenario',
      vus: 1,
      duration: '20s',
      startTime: '5s',
    },
  },
};

// ========================
// HELPERS
// ========================

function apiUrl(path) {
  return `${API_BASE_URL.replace(/\/+$/, '')}${path}`;
}

function defaultHeaders(token) {
  const headers = { 'Content-Type': 'application/json' };
  if (token) headers['Authorization'] = `Bearer ${token}`;
  return headers;
}

// Simple random string generator for test usernames/emails
function randomString(length = 10) {
  const chars = 'abcdefghijklmnopqrstuvwxyz0123456789';
  let out = '';
  for (let i = 0; i < length; i++) {
    out += chars.charAt(Math.floor(Math.random() * chars.length));
  }
  return out;
}

function buildRoomWsUrl(roomId, token) {
  const httpUrl = API_BASE_URL.replace(/\/+$/, '');
  const wsProtocol = httpUrl.startsWith('https') ? 'wss' : 'ws';
  const withoutProtocol = httpUrl.replace(/^https?:\/\//, '');
  return `${wsProtocol}://${withoutProtocol}/ws/rooms/${roomId}?token=${encodeURIComponent(
    token,
  )}`;
}

// ------------------------
// USER CREATION & LOGIN
// ------------------------
//
// No fixed identifier/password.
// Each VU creates a disposable user:
//   - POST /auth/register
//   - POST /auth/login
//   - Login response may be JSON OR plain text token.
//

function extractTokenFromLoginResponse(res) {
  // 1) Try JSON: { token } or { access_token }
  try {
    const body = res.json();
    if (body && (body.token || body.access_token)) {
      return body.token || body.access_token;
    }
  } catch (_) {
    // ignore, will try plain text
  }

  // 2) Fallback: raw body as token string (plain text)
  if (typeof res.body === 'string' && res.body.trim().length > 0) {
    // Trim quotes if backend returns a quoted string
    return res.body.trim().replace(/^"|"$/g, '');
  }

  return null;
}

function createTestUser() {
  const username = `k6_${randomString(6)}`;
  const email = `${username}@test.local`;
  const password = randomString(12);

  // 1) Register user
  const registerRes = http.post(
    apiUrl('/auth/register'),
    JSON.stringify({
      username,
      email,
      password,
    }),
    {
      headers: defaultHeaders(),
      tags: { endpoint: 'auth_register' },
    },
  );

  // It's OK if registration returns 200/201
  check(registerRes, {
    'register status is 200/201': (r) => r.status === 200 || r.status === 201,
  });

  // 2) Login
  const loginRes = http.post(
    apiUrl('/auth/login'),
    JSON.stringify({
      identifier: email,
      password,
    }),
    {
      headers: defaultHeaders(),
      tags: { endpoint: 'auth_login' },
    },
  );

  check(loginRes, {
    'login status is 200': (r) => r.status === 200,
  });

  const token = extractTokenFromLoginResponse(loginRes);
  if (!token) {
    throw new Error('Could not extract JWT token from login response');
  }

  return { token, username, email, password };
}

// ------------------------
// ROOM MANAGEMENT HELPERS
// ------------------------

function createRoom(token, roomName) {
  const payload = JSON.stringify({
    name: roomName,
    visibility: 'public',
    password: null,
  });

  const res = http.post(apiUrl('/rooms'), payload, {
    headers: defaultHeaders(token),
    tags: { endpoint: 'create_room' },
  });

  check(res, {
    'create room status is 200/201': (r) => r.status === 200 || r.status === 201,
  });

  let room;
  try {
    room = res.json();
  } catch (_) {
    throw new Error('Could not parse create room response as JSON');
  }

  // IMPORTANT:
  // Creator user is automatically joined to the room (per your rules).
  return room.id;
}

function joinRoom(token, roomId) {
  const res = http.post(
    apiUrl(`/rooms/${roomId}/members`),
    JSON.stringify({}),
    {
      headers: defaultHeaders(token),
      tags: { endpoint: 'join_room' },
    },
  );

  http_room_join_latency.add(res.timings.duration);

  // Join is allowed only if room exists & user not already in it.
  check(res, {
    'join room is 200/201': (r) => r.status === 200 || r.status === 201,
  });

  return res;
}

function leaveRoom(token, roomId) {
  // You can only leave if you are already a member.
  const res = http.del(apiUrl(`/rooms/${roomId}/members/me`), null, {
    headers: defaultHeaders(token),
    tags: { endpoint: 'leave_room' },
  });

  http_room_leave_latency.add(res.timings.duration);

  check(res, {
    'leave room is 200/204': (r) => r.status === 200 || r.status === 204,
  });

  return res;
}

// ------------------------
// WS RECEIVER CONNECTION
// ------------------------
//
// Receivers:
// - Join the room
// - Connect WS
// - Do NOT send messages
// - Only listen for messages sent by the sender user
// - Measure latency using timestamp embedded in content:
//      content = "LAT_TEST:<timestamp_ms>"
//

function connectAsReceiver(token, roomId, variantTag) {
  // User must join the room first to be allowed to receive messages
  joinRoom(token, roomId);

  const wsUrl = buildRoomWsUrl(roomId, token);

  ws.connect(wsUrl, {}, function(socket) {
    socket.on('open', function() {
      // Receiver doesn’t send anything
    });

    socket.on('message', function(data) {
      // Data is expected to be JSON with Message shape:
      // { id, roomId, senderId, content, createdAt }
      try {
        const msg = JSON.parse(data);

        if (!msg || typeof msg.content !== 'string') {
          return;
        }

        // Only measure messages that are part of test traffic
        if (msg.content.startsWith('LAT_TEST:')) {
          const parts = msg.content.split(':');
          if (parts.length >= 2) {
            const sendTs = parseInt(parts[1], 10);
            if (!isNaN(sendTs)) {
              const recvTs = Date.now();
              const latencyMs = recvTs - sendTs;
              message_delivery_latency.add(latencyMs, { variant: variantTag });
            }
          }
        }
      } catch (_) {
        // Ignore malformed messages
      }
    });

    socket.on('error', function() {
      // Let k6 record the WS error; nothing extra needed
    });

    // Keep connection open for the duration of scenario; safety timeout
    socket.setTimeout(function() {
      socket.close();
    }, 20000); // 20s per connection
  });
}

// ========================
// SETUP
// ========================
//
// 1) Create a single "sender" user.
// 2) That user creates a room (creator is auto-joined).
// 3) Return { roomId, senderToken } to scenarios.
//

export function setup() {
  const { token: senderToken } = createTestUser();
  const testRoomName = 'k6-latency-room';

  const roomId = createRoom(senderToken, testRoomName);

  return { roomId, senderToken };
}

// ========================
// SCENARIO: HTTP ENDPOINT LATENCY
// ========================
//
// - Each iteration:
//   - Create a fresh user
//   - Join existing test room
//   - Call /me, /rooms, /rooms/{id}/messages GET
//   - Send a message
//   - Leave room
//

export function httpEndpointLatencyScenario(data) {
  const { roomId } = data;

  const { token } = createTestUser();

  // Join room (required for room-specific endpoints)
  joinRoom(token, roomId);

  // /me
  const meRes = http.get(apiUrl('/me'), {
    headers: defaultHeaders(token),
    tags: { endpoint: 'me' },
  });
  http_me_latency.add(meRes.timings.duration);
  check(meRes, {
    '/me is 200': (r) => r.status === 200,
  });

  // /rooms (rooms user belongs to)
  const roomsRes = http.get(apiUrl('/rooms'), {
    headers: defaultHeaders(token),
    tags: { endpoint: 'rooms' },
  });
  http_rooms_latency.add(roomsRes.timings.duration);
  check(roomsRes, {
    '/rooms is 200': (r) => r.status === 200,
  });

  // /rooms/{id}/messages GET
  const msgRes = http.get(apiUrl(`/rooms/${roomId}/messages?page=1&page_size=20`), {
    headers: defaultHeaders(token),
    tags: { endpoint: 'room_messages_get' },
  });
  http_messages_get_latency.add(msgRes.timings.duration);
  check(msgRes, {
    '/rooms/{id}/messages is 200': (r) => r.status === 200,
  });

  // Send a message via HTTP POST
  const postRes = http.post(
    apiUrl(`/rooms/${roomId}/messages`),
    JSON.stringify({ content: `HTTP_LAT_TEST:${Date.now()}` }),
    {
      headers: defaultHeaders(token),
      tags: { endpoint: 'post_room_message', variant: 'http_latency' },
    },
  );
  http_messages_post_latency.add(postRes.timings.duration);
  check(postRes, {
    'POST /rooms/{id}/messages is 200/201': (r) => r.status === 200 || r.status === 201,
  });

  // Leave room (only valid because we joined before)
  leaveRoom(token, roomId);

  sleep(1);
}

// ========================
// SCENARIO: WS RECEIVER (SINGLE USER)
// ========================
//
// 1 receiver, many messages from sender.
// Measures message_delivery_latency with variant "single_receiver".
//

export function wsReceiverSingleScenario(data) {
  const { roomId } = data;
  const { token } = createTestUser();

  connectAsReceiver(token, roomId, 'single_receiver');
  sleep(1);
}

// ========================
// SCENARIO: WS RECEIVERS (MANY USERS)
// ========================
//
// 100 receivers, all listening to messages from the single sender.
// Measures message_delivery_latency with variant "many_receivers".
//

export function wsReceiversManyScenario(data) {
  const { roomId } = data;
  const { token } = createTestUser();

  connectAsReceiver(token, roomId, 'many_receivers');
  sleep(1);
}

// ========================
// SCENARIO: WS SENDER
// ========================
//
// Single user that sends messages via HTTP POST only.
// This user:
//   - is the room creator (from setup), thus already joined.
//   - never connects via WS.
//   - sends messages with content "LAT_TEST:<timestamp_ms>" so receivers can compute latency.
//

export function wsSenderScenario(data) {
  const { roomId, senderToken } = data;

  // Send a message every iteration
  const sendTs = Date.now();
  const content = `LAT_TEST:${sendTs}`;

  const res = http.post(
    apiUrl(`/rooms/${roomId}/messages`),
    JSON.stringify({ content }),
    {
      headers: defaultHeaders(senderToken),
      tags: { endpoint: 'post_room_message', variant: 'ws_sender' },
    },
  );

  http_messages_post_latency.add(res.timings.duration, { variant: 'ws_sender' });

  check(res, {
    'sender POST /rooms/{id}/messages is 200/201': (r) => r.status === 200 || r.status === 201,
  });

  sleep(1);
}

// ========================
// SUMMARY (HTML REPORT)
// ========================

export function handleSummary(data) {
  return {
    'nebula-latency-report.html': htmlReport(data, {
      title: 'Nebula Backend Latency Report',
      theme: 'default',
    }),
  };
}

